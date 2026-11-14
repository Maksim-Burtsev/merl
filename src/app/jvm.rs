use super::{
    App, Candidate, HashMap, HashSet, Hit, Kind, Path, PathBuf, Reason, Regex, is_word, search,
    whole_at,
};

impl App {
    pub(super) fn enum_constant(
        &self,
        kind: Kind,
        here: &Path,
        owner: &str,
        word: &str,
    ) -> Vec<Candidate> {
        let owners = search::def_patterns(kind, owner).join("|");
        let declared = self.project_definitions(kind, here, owner, &owners);
        self.truncated.set(false);
        let [decl] = declared.as_slice() else {
            return Vec::new();
        };
        let o = regex::escape(owner);
        let is_enum = Regex::new(&format!(r"\benum\s+(?:class\s+)?{o}\b"))
            .is_ok_and(|re| re.is_match(&decl.text));
        let Some(text) = self.text_of(&decl.path).filter(|_| is_enum) else {
            return Vec::new();
        };
        let package = |t: &str| {
            let re = Regex::new(r"^\s*package\s+([\w.]+)").expect("a fixed pattern");
            t.lines()
                .find_map(|l| re.captures(l).map(|c| c[1].to_owned()))
                .unwrap_or_default()
        };
        let import = Regex::new(&format!(r"^\s*import\s+([\w.]+)\.({o}|\*)\s*;?\s*$"))
            .expect("an escaped name keeps the pattern valid");
        let home = package(&text);
        let imports: Vec<(String, bool)> = (self.buf.lines.iter())
            .filter_map(|l| import.captures(l))
            .map(|c| (c[1].to_owned(), &c[2] == "*"))
            .collect();
        let elsewhere = imports.iter().any(|(p, all)| !all && *p != home);
        let sees =
            package(&self.buf.lines.join("\n")) == home || imports.iter().any(|(p, _)| *p == home);
        if elsewhere || !sees {
            return Vec::new();
        }
        self.jvm_enum_constant(decl, word)
    }

    pub(super) fn jvm_hidden_elsewhere(&self, file: &Path, hit: &Hit) -> bool {
        if hit.path == file || file.extension().is_none_or(|e| e != "java") {
            return false;
        }
        let Some(text) = self.text_of(&hit.path) else {
            return false;
        };
        let lines: Vec<&str> = text.lines().collect();
        let owners: Vec<String> = search::jvm_enclosing_types(&text, hit.line1)
            .into_iter()
            .filter_map(|l| lines.get(l - 1).and_then(|l| search::jvm_type_name(l)))
            .collect();
        if owners.is_empty() {
            return false;
        }
        let names = owners
            .iter()
            .map(|o| regex::escape(o))
            .collect::<Vec<_>>()
            .join("|");
        let reach = format!(
            r"\b(?:extends|implements)\b.*\b(?:{names})\b|(?:\b(?:class|object|interface)\b|^\s*\)).*:[^=]*\b(?:{names})\b|\bimport\b.*\b(?:{names})\s*\.|\bnew\s+(?:[\w.]+\.)?(?:{names})\s*(?:<.*>)?\s*\(.*\)\s*\{{|^\s*[:,]?\s*(?:[\w.]+\.)?(?:{names})\s*(?:<.*>)?\s*(?:\(.*\))?\s*(?:,|\{{|$)"
        );
        self.project_grep(Kind::Jvm, file, &reach).is_empty()
    }

    pub(super) fn java_field_owner(&self, here: &Path, text: &str, name: &str) -> Option<Hit> {
        let line = self.line + 1;
        let narrowed = Regex::new(&format!(r"\b{}\s+instanceof\b", regex::escape(name))).ok()?;
        (here.extension().is_some_and(|e| e == "java")
            && search::path_bindings(Kind::Jvm, here, text, line, name).is_empty()
            && search::jvm_bindings_at(here, text, line, self.col, name).is_empty()
            && !narrowed.is_match(text))
        .then_some(())?;
        let lines: Vec<&str> = text.lines().collect();
        let enclosing = search::jvm_enclosing_types(text, line);
        let top = |t: &str, l| search::jvm_enclosing_types(t, l).last().copied();
        let seen = |h: &Hit| {
            let words: Vec<&str> = h.text.split_whitespace().collect();
            if words.contains(&"private") {
                h.path == here && top(text, h.line1) == enclosing.last().copied()
            } else {
                words.iter().any(|w| ["public", "protected"].contains(w))
                    || self.text_of(&h.path).and_then(|t| search::jvm_package(&t))
                        == search::jvm_package(text)
            }
        };
        enclosing
            .iter()
            .map(|&d| Hit {
                deleted: None,
                path: here.to_path_buf(),
                line1: d,
                byte_col: None,
                text: lines[d - 1].to_owned(),
            })
            .find(|decl| {
                self.jvm_hierarchy_member(decl, name, 0, &mut Vec::new(), false)
                    .iter()
                    .any(|h| {
                        search::jvm_type_name(&h.text).is_none()
                            && !search::java_declares_method(&h.text, name)
                            && seen(h)
                    })
            })
    }

    pub(super) fn jvm_imported(
        &self,
        text: &str,
        chain: &[String],
        word: &str,
        range: std::ops::Range<usize>,
        cursor_on_import: bool,
    ) -> Option<Vec<Candidate>> {
        let found = self.jvm_imported_all(text, chain, word, cursor_on_import)?;
        let here = self.rel_current()?;
        Some(self.jvm_fit_candidates(&here, word, range, chain, found))
    }

    pub(super) fn jvm_binds(&self, import: &search::JvmImport, name: &str) -> bool {
        import.bound_name == name
            && (!import.selected
                || self.jvm_packages().is_some_and(|packages| {
                    (1..import.path.len())
                        .any(|n| packages.contains_key(&import.path[..n].join(".")))
                }))
    }

    pub(super) fn jvm_value_named(&self, here: &Path, text: &str, name: &str) -> bool {
        let line1 = self.line + 1;
        let value = |l: usize| {
            search::scala(here)
                || text
                    .lines()
                    .nth(l - 1)
                    .is_some_and(|l| !search::java_declares_method(l, name))
        };
        !search::path_bindings(Kind::Jvm, here, text, line1, name).is_empty()
            || search::jvm_class_parameter(text, line1, name, search::scala(here))
            || search::jvm_enclosing_types(text, line1)
                .into_iter()
                .any(|d| {
                    search::jvm_members_of(here, text, d, name)
                        .into_iter()
                        .any(value)
                })
    }

    pub(super) fn jvm_package_sees(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        there: &Path,
    ) -> bool {
        let package = |p: &Path| {
            self.text_of(p)
                .and_then(|t| search::jvm_package(&t))
                .unwrap_or_default()
        };
        let declared = package(there);
        there == here
            || declared == package(here)
            || search::jvm_imports(here, text).iter().any(|i| {
                let path = i.path.join(".");
                path == format!("{declared}.{word}") || path == format!("{declared}.*")
            })
    }

    pub(super) fn jenkins_step(&self, here: &Path, word: &str) -> Vec<Candidate> {
        static CALL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^\s*(?:def|void)\s+call\s*\(").expect("the regex is a literal")
        });
        if !search::groovy(here) {
            return Vec::new();
        }
        let file = format!("{word}.groovy");
        self.files
            .iter()
            .filter(|p| p.ends_with(Path::new("vars").join(&file)))
            .map(|p| {
                let text = self.text_of(p).unwrap_or_default();
                let (line, text) = text
                    .lines()
                    .enumerate()
                    .find(|(_, l)| CALL.is_match(l))
                    .or_else(|| text.lines().enumerate().next())
                    .map_or((1, String::new()), |(i, l)| (i + 1, l.to_owned()));
                Candidate {
                    hit: Hit {
                        deleted: None,
                        path: p.clone(),
                        line1: line,
                        byte_col: None,
                        text,
                    },
                    reason: Reason::ByName,
                }
            })
            .collect()
    }

    pub(super) fn jvm_seen(&mut self, here: &Path, hits: Vec<Hit>, behind: bool) -> Vec<Hit> {
        let mut all = hits.len();
        let mut texts: HashMap<PathBuf, Option<String>> = HashMap::new();
        let mut literals: HashMap<PathBuf, Vec<bool>> = HashMap::new();
        let mut seen_local = false;
        let kept: Vec<Hit> = hits
            .into_iter()
            .filter(|h| {
                if h.path != here && search::jvm_private(&h.text) {
                    return false;
                }
                let text = texts
                    .entry(h.path.clone())
                    .or_insert_with(|| self.text_of(&h.path));
                if !h.text.starts_with([' ', '\t'])
                    || (search::groovy(&h.path) && search::gradle_global(&h.text))
                {
                    let t = text.as_deref().unwrap_or("");
                    return !search::groovy_script_local(&h.path, t, h.line1)
                        || (!behind
                            && h.path == here
                            && search::groovy_script_sees(here, t, h.line1, self.line + 1));
                }
                match text
                    .as_deref()
                    .and_then(|t| search::jvm_local_block(t, h.line1))
                {
                    None => true,
                    Some(end) => {
                        let seen =
                            !behind && h.path == here && (h.line1..=end).contains(&(self.line + 1));
                        seen_local |= seen;
                        let literal = literals
                            .entry(h.path.clone())
                            .or_insert_with(|| {
                                search::path_literal_lines(
                                    Kind::Jvm,
                                    &h.path,
                                    text.as_deref().unwrap_or(""),
                                )
                            })
                            .get(h.line1 - 1)
                            .copied()
                            .unwrap_or(false);
                        all -= usize::from(!seen && literal);
                        seen
                    }
                }
            })
            .collect();
        self.offer_only |= kept.len() == 1 && kept.len() < all && !seen_local;
        kept
    }

    pub(super) fn jvm_fit_candidates(
        &self,
        here: &Path,
        word: &str,
        range: std::ops::Range<usize>,
        chain: &[String],
        found: Vec<Candidate>,
    ) -> Vec<Candidate> {
        let hits: Vec<Hit> = found.iter().map(|c| c.hit.clone()).collect();
        let end = range.end;
        let kept = self.jvm_use_fit(here, word, range, chain, hits);
        let kept = self.jvm_overloads(here, word, end, kept);
        let fit: Vec<Candidate> = found
            .iter()
            .filter(|c| {
                kept.iter()
                    .any(|h| (&h.path, h.line1) == (&c.hit.path, c.hit.line1))
            })
            .cloned()
            .collect();
        if fit.is_empty() { found } else { fit }
    }

    pub(super) fn jvm_imported_all(
        &self,
        text: &str,
        chain: &[String],
        word: &str,
        cursor_on_import: bool,
    ) -> Option<Vec<Candidate>> {
        let here = self.rel_current().unwrap_or_default();
        let imports = search::jvm_imports_at(&here, text, self.line + 1);
        let first = chain.first().map_or(word, String::as_str);
        let bound = imports
            .iter()
            .find(|i| self.jvm_binds(i, first))
            .map(|i| &i.path);
        let capital = word.starts_with(|c: char| c.is_ascii_uppercase());
        let wildcards: Vec<&Vec<String>> = imports
            .iter()
            .filter(|i| i.bound_name == "*")
            .map(|i| &i.path)
            .collect();
        let unbound = chain.is_empty() && (capital || !wildcards.is_empty());
        if !cursor_on_import && bound.is_none() && !unbound {
            return None;
        }
        let packages = self.jvm_packages()?;
        let longest_project_package_and_rest = |path: &[String]| {
            (1..path.len()).rev().find_map(|n| {
                let files = packages.get(&path[..n].join("."))?;
                Some((files, path[n..].to_vec()))
            })
        };
        if cursor_on_import {
            let mut path = chain.to_vec();
            path.push(word.to_owned());
            let Some((files, names)) = longest_project_package_and_rest(&path) else {
                return Some(Vec::new());
            };
            let found = self.jvm_declared(files, &names);
            return (!found.is_empty()).then_some(found);
        }
        if let Some(path) = bound {
            if let Some(here) = self.rel_current()
                && !Self::jvm_nested_in_scope(&here, text, first, self.line + 1).is_empty()
            {
                return None;
            }
            let scala = search::scala(&here);
            let package = (packages.get(&path.join(".")))
                .filter(|_| scala)
                .map(|files| (files, Vec::new()));
            let Some((files, mut names)) =
                package.or_else(|| longest_project_package_and_rest(path))
            else {
                let relative =
                    (path.first()).is_some_and(|p| p.starts_with(|c: char| c.is_ascii_uppercase()));
                return (!(scala && relative)).then(Vec::new);
            };
            if !chain.is_empty() {
                names.extend(chain[1..].iter().cloned());
                names.push(word.to_owned());
            }
            let found = self.jvm_declared(files, &names);
            return (!found.is_empty()).then_some(found);
        }
        let own = search::jvm_package(text);
        if capital && let Some(files) = own.as_ref().and_then(|p| packages.get(p)) {
            let found = self.jvm_declared(files, &[word.to_owned()]);
            if !found.is_empty() {
                let package = own.unwrap_or_default();
                return Some(
                    found
                        .into_iter()
                        .map(|c| Candidate {
                            reason: Reason::Path(package.clone()),
                            ..c
                        })
                        .collect(),
                );
            }
        }
        let mut found = Vec::new();
        for path in wildcards {
            let base = &path[..path.len() - 1];
            let names = match packages.get(&base.join(".")) {
                Some(files) if capital => Some((files, vec![word.to_owned()])),
                Some(_) => None,
                None => longest_project_package_and_rest(base).map(|(files, mut names)| {
                    names.push(word.to_owned());
                    (files, names)
                }),
            };
            if let Some((files, names)) = names {
                found.extend(self.jvm_declared(files, &names));
            }
        }
        (!found.is_empty()).then_some(found)
    }

    pub(super) fn jvm_members(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let lines: Vec<&str> = text.lines().collect();
        let candidate = |path: &Path, lines: &[&str], line: usize, reason: Reason| Candidate {
            hit: Hit {
                deleted: None,
                path: path.to_path_buf(),
                line1: line,
                byte_col: None,
                text: lines[line - 1].to_owned(),
            },
            reason,
        };
        let kotlin = here.extension().is_some_and(|e| e == "kt" || e == "kts");
        if kotlin
            && let Some(line) =
                search::kotlin_initializer_parameter(text, self.line + 1, before, word)
        {
            return Some(vec![candidate(here, &lines, line, Reason::Local)]);
        }
        if kotlin
            && word.starts_with(|c: char| c.is_ascii_lowercase())
            && let Some(found) = self.kotlin_scope_member(here, text, before, word)
        {
            return Some(found);
        }
        if search::groovy(here)
            && word.starts_with(|c: char| c.is_ascii_lowercase())
            && search::jvm_receiver_lambda(text, self.line + 1, before)
        {
            return None;
        }
        if let Some(header) = search::jvm_anonymous_header(before) {
            let found = self.jvm_anonymous_inherited(here, text, &header, word);
            if !found.is_empty() {
                return Some(found);
            }
        }
        let types = search::jvm_enclosing_types(text, self.line + 1);
        let mut found: Vec<Candidate> = Vec::new();
        for (i, &decl) in types.iter().enumerate() {
            let reason = match search::jvm_type_name(lines[decl - 1]) {
                Some(name) => Reason::Path(name),
                None => Reason::Local,
            };
            found = search::jvm_members_of(here, text, decl, word)
                .into_iter()
                .map(|line| candidate(here, &lines, line, reason.clone()))
                .collect();
            if found.is_empty() && i + 1 < types.len() {
                found = self.jvm_nested_inherited(here, text, lines[decl - 1], decl, word);
            }
            if !found.is_empty() {
                break;
            }
        }
        let inner = types
            .iter()
            .find(|&&d| search::jvm_type_name(lines[d - 1]).is_some());
        if found.is_empty()
            && let Some(&decl) = inner
        {
            let mut known = true;
            for base in search::jvm_bases(text, decl, search::scala(here)) {
                match self.jvm_base_members(here, &base, word) {
                    Some(f) => found.extend(f),
                    None => known = false,
                }
            }
            if found.is_empty() && known {
                for base in search::java_implements(text, decl) {
                    found.extend(self.jvm_base_members(here, &base, word).unwrap_or_default());
                }
            }
        }
        if found.is_empty()
            && types.is_empty()
            && kotlin
            && word.starts_with(|c: char| c.is_ascii_lowercase())
            && let Some(ext) = search::kotlin_extension_at(text, self.line + 1, before)
            && let super::jvm_typed::JvmType::Project(decl) =
                self.jvm_type_at(here, text, &ext.receiver)
        {
            let reason = Reason::Receiver(format!("this: {}", ext.receiver));
            found = self
                .jvm_hierarchy_member(&decl, word, 0, &mut Vec::new(), false)
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect();
        }
        let on_it = found
            .iter()
            .any(|c| c.hit.path == here && c.hit.line1 == self.line + 1);
        (!found.is_empty() && !on_it).then_some(found)
    }

    fn jvm_nested_inherited(
        &self,
        here: &Path,
        text: &str,
        header: &str,
        decl: usize,
        word: &str,
    ) -> Vec<Candidate> {
        let Some(name) = search::jvm_type_name(header) else {
            return self.jvm_anonymous_inherited(here, text, header, word);
        };
        let ty = Hit {
            deleted: None,
            path: here.to_path_buf(),
            line1: decl,
            byte_col: None,
            text: header.to_owned(),
        };
        self.jvm_hierarchy_found(&ty, &name, word)
    }

    fn jvm_anonymous_inherited(
        &self,
        here: &Path,
        text: &str,
        header: &str,
        word: &str,
    ) -> Vec<Candidate> {
        let mut found = Vec::new();
        for base in search::jvm_anonymous_bases(header) {
            if let super::jvm_typed::JvmType::Project(b) = self.jvm_type_at(here, text, &base) {
                found.extend(self.jvm_hierarchy_found(&b, &base, word));
            }
        }
        found
    }

    fn jvm_hierarchy_found(&self, ty: &Hit, name: &str, word: &str) -> Vec<Candidate> {
        self.jvm_hierarchy_member(ty, word, 0, &mut Vec::new(), false)
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Path(name.to_owned()),
            })
            .collect()
    }

    fn jvm_base_members(&self, here: &Path, base: &str, word: &str) -> Option<Vec<Candidate>> {
        let pattern = search::def_patterns(Kind::Jvm, base).join("|");
        let cut = self.truncated.get();
        let declared: Vec<Hit> = self
            .project_definitions(Kind::Jvm, here, base, &pattern)
            .into_iter()
            .filter(|h| search::jvm_type_name(&h.text).as_deref() == Some(base))
            .collect();
        self.truncated.set(cut);
        let [hit] = declared.as_slice() else {
            return None;
        };
        let t = self.text_of(&hit.path)?;
        let lines: Vec<&str> = t.lines().collect();
        Some(
            search::jvm_members_of(&hit.path, &t, hit.line1, word)
                .into_iter()
                .map(|line| Candidate {
                    hit: Hit {
                        deleted: None,
                        path: hit.path.clone(),
                        line1: line,
                        byte_col: None,
                        text: lines[line - 1].to_owned(),
                    },
                    reason: Reason::Path(base.to_owned()),
                })
                .collect(),
        )
    }

    fn jvm_overloads(&self, here: &Path, word: &str, end: usize, hits: Vec<Hit>) -> Vec<Hit> {
        if hits.len() < 2 {
            return hits;
        }
        let Some(call) = self.jvm_call_here(here, end) else {
            return hits;
        };
        let parts = |h: &Hit| search::jvm_parameter_parts(&self.text_of(&h.path)?, h.line1, word);
        let mut hits = hits;
        if Self::plain_arguments(&call) {
            let fit: Vec<Hit> = hits
                .iter()
                .filter(|h| self.jvm_fits(here, &call, word, h))
                .cloned()
                .collect();
            if !fit.is_empty() {
                hits = fit;
            }
        }
        let literals: Option<Vec<&str>> = call
            .arguments
            .iter()
            .map(|a| search::jvm_literal_argument(a))
            .collect();
        let Some(args) = literals.filter(|a| !a.is_empty() && !call.trailing_lambda) else {
            return hits;
        };
        search::literal_overloads(hits, &args, |h| {
            Some(
                parts(h)?
                    .iter()
                    .map(|t| search::jvm_literal_parameter(t))
                    .collect(),
            )
        })
    }

    fn jvm_call_here(&self, here: &Path, end: usize) -> Option<search::JvmCall> {
        let line = self.line_str();
        let called = line.get(end..).is_some_and(|a| a.starts_with('('));
        if search::groovy(here) || !called || line.contains("\"\"\"") {
            return None;
        }
        search::jvm_call(&self.buf.lines, self.line, end)
    }

    fn plain_arguments(call: &search::JvmCall) -> bool {
        !call
            .arguments
            .iter()
            .any(|a| a.starts_with('*') || a.contains('<'))
    }

    fn jvm_fits(&self, here: &Path, call: &search::JvmCall, word: &str, h: &Hit) -> bool {
        let kotlin = here.extension().is_none_or(|e| e != "java");
        self.text_of(&h.path)
            .and_then(|t| search::jvm_parameter_parts(&t, h.line1, word))
            .is_none_or(|p| search::jvm_takes_call(&p, call, kotlin))
    }

    pub(super) fn jvm_by_arguments(
        &self,
        here: &Path,
        word: &str,
        end: usize,
        found: &mut Vec<Candidate>,
    ) {
        let java = here.extension().is_some_and(|e| e == "java");
        if let Some(call) = self.jvm_call_here(here, end)
            && java
            && Self::plain_arguments(&call)
            && found
                .iter()
                .any(|c| self.jvm_fits(here, &call, word, &c.hit))
        {
            found.retain(|c| self.jvm_fits(here, &call, word, &c.hit));
        }
    }

    pub(super) fn jvm_constructors(&self, types: &[Candidate], word: &str) -> Vec<Candidate> {
        let Ok(re) = Regex::new(&search::def_patterns(Kind::Jvm, word).join("|")) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for c in types {
            let Some(text) = self.text_of(&c.hit.path) else {
                continue;
            };
            let lines: Vec<&str> = text.lines().collect();
            let literal = search::path_literal_lines(Kind::Jvm, &c.hit.path, &text);
            let inside = |line: usize| {
                re.is_match(lines[line - 1])
                    && !literal.get(line - 1).copied().unwrap_or(false)
                    && search::jvm_enclosing_types(&text, line).first() == Some(&c.hit.line1)
            };
            for line in (c.hit.line1 + 1..=lines.len()).filter(|&l| inside(l)) {
                out.push(Candidate {
                    hit: Hit {
                        deleted: None,
                        path: c.hit.path.clone(),
                        line1: line,
                        byte_col: None,
                        text: lines[line - 1].to_owned(),
                    },
                    reason: c.reason.clone(),
                });
            }
        }
        out
    }

    pub(super) fn jvm_packages(&self) -> Option<HashMap<String, Vec<PathBuf>>> {
        let jvm = |p: &Path| search::kind_of(p) == Some(Kind::Jvm);
        let hits = self
            .grep(r"^[ \t]*package\s+[\w.]*\w", false, false, jvm)
            .ok()?;
        if hits.len() >= search::MAX_HITS {
            return None;
        }
        let mut out: HashMap<String, Vec<PathBuf>> = HashMap::new();
        let mut seen = HashSet::new();
        for h in hits {
            if seen.insert(h.path.clone())
                && let Some(p) = search::jvm_package(&h.text)
            {
                out.entry(p).or_default().push(h.path);
            }
        }
        Some(out)
    }

    pub(super) fn jvm_declared(&self, files: &[PathBuf], names: &[String]) -> Vec<Candidate> {
        let Some(name) = names.last() else {
            return Vec::new();
        };
        let files: Vec<PathBuf> = files
            .iter()
            .filter(|f| {
                f.extension().is_none_or(|e| e != "java")
                    || f.file_stem() == Some(names[0].as_ref())
            })
            .cloned()
            .collect();
        if files.is_empty() {
            return Vec::new();
        }
        let pattern = search::def_patterns(Kind::Jvm, name).join("|");
        let hits = self.grep_in(&pattern, &files);
        let within = (names.len() > 1).then(|| names.join("."));
        let constructor = Some(format!("{}.{name}", names.join(".")));
        self.declaring(Kind::Jvm, name, hits)
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path)
                    .map(|t| search::qualified(Kind::Jvm, &t, h.line1, name))
                    .is_some_and(|q| q == within || (q.is_some() && q == constructor))
            })
            .map(|hit| Candidate {
                reason: Reason::Import(hit.path.display().to_string()),
                hit,
            })
            .collect()
    }
}

const KOTLIN_KEYWORDS: &[&str] = &[
    "in",
    "is",
    "as",
    "else",
    "return",
    "val",
    "var",
    "fun",
    "class",
    "object",
    "interface",
    "if",
    "when",
    "while",
    "for",
    "do",
    "try",
    "catch",
    "finally",
    "throw",
    "import",
    "package",
    "typealias",
    "by",
    "where",
    "get",
    "set",
    "init",
    "constructor",
    "private",
    "public",
    "protected",
    "internal",
    "override",
    "open",
    "abstract",
    "lateinit",
    "const",
    "data",
    "enum",
    "sealed",
    "inline",
    "suspend",
    "operator",
    "infix",
    "companion",
    "annotation",
    "inner",
    "value",
    "out",
    "reified",
    "vararg",
    "noinline",
    "crossinline",
    "external",
    "tailrec",
    "final",
    "expect",
    "actual",
];

fn arguments(lines: &[String], at: usize, open_byte: usize) -> Option<(usize, bool)> {
    let mut depth = 0usize;
    let (mut commas, mut any, mut angle) = (0, false, false);
    let mut quote: Option<&[u8]> = None;
    for (n, line) in lines.iter().enumerate().skip(at).take(60) {
        let from = if n == at { open_byte } else { 0 };
        let b = line.as_bytes();
        let mut i = from;
        while i < b.len() {
            let rest = &b[i..];
            if let Some(q) = quote {
                if rest.starts_with(b"\\") {
                    i += 2;
                    continue;
                }
                if rest.starts_with(q) {
                    i += q.len();
                    quote = None;
                    continue;
                }
                i += 1;
                continue;
            }
            let c = b[i];
            if rest.starts_with(b"\"\"\"") {
                quote = Some(b"\"\"\"");
                any |= depth >= 1;
                i += 3;
                continue;
            }
            match c {
                b'"' => quote = Some(b"\""),
                b'\'' => quote = Some(b"'"),
                b'/' if rest.starts_with(b"//") => break,
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some((if any { commas + 1 } else { 0 }, angle));
                    }
                }
                b',' if depth == 1 => commas += 1,
                b'<' => angle = true,
                _ => {}
            }
            if depth >= 1 && !(depth == 1 && c == b'(') && !c.is_ascii_whitespace() {
                any = true;
            }
            i += 1;
        }
        if quote != Some(b"\"\"\"") {
            quote = None;
        }
    }
    None
}

static MAP_ARGUMENT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\(\s*(?:\[\s*(?:\w+\s*:|:\s*\])|\w+\s*:[^:])").expect("the regex is a literal")
});

impl App {
    pub(super) fn jvm_use_fit(
        &self,
        here: &Path,
        word: &str,
        range: std::ops::Range<usize>,
        chain: &[String],
        hits: Vec<Hit>,
    ) -> Vec<Hit> {
        let line = self.line_str();
        let (Some(before), Some(after)) = (line.get(..range.start), line.get(range.end..)) else {
            return hits;
        };
        let java = here.extension().is_some_and(|e| e == "java");
        let groovy = search::groovy(here);
        if !java && !groovy {
            return if Self::kotlin_infix(before, after, word) {
                hits.into_iter()
                    .filter(|h| Regex::new(r"\binfix\b").is_ok_and(|re| re.is_match(&h.text)))
                    .collect()
            } else {
                hits
            };
        }
        let w = regex::escape(word);
        let class_re = Regex::new(&format!(r"\b(?:class|record|enum|interface)\s+{w}\b"))
            .expect("an escaped name keeps the pattern valid");
        let call_re =
            Regex::new(&format!(r"\b{w}\s*\(")).expect("an escaped name keeps the pattern valid");
        let is_class = |h: &Hit| class_re.is_match(&h.text);
        let java_or_groovy =
            |h: &Hit| h.path.extension().is_some_and(|e| e == "java") || search::groovy(&h.path);
        let constructor = |h: &Hit| {
            java_or_groovy(h)
                && !is_class(h)
                && word.starts_with(char::is_uppercase)
                && call_re.is_match(&h.text)
        };
        let constructed = before
            .trim_end()
            .strip_suffix("new")
            .is_some_and(|b| !b.ends_with(is_word));
        let mut hits = hits;
        let has_class = hits.iter().any(is_class);
        let has_constructor = hits.iter().any(constructor);
        if constructed && has_constructor {
            hits.retain(|h| !is_class(h));
        } else if !constructed && has_class && has_constructor {
            hits.retain(|h| !constructor(h));
        }
        let call = after.starts_with('(');
        if !call || (groovy && !constructed) {
            return hits;
        }
        if !constructed && matches!(word, "values" | "valueOf") {
            let found = self.jvm_enum_of(here, chain);
            if !found.is_empty() {
                return found;
            }
        }
        if !constructed {
            let component = Regex::new(&format!(
                r"\brecord\s+\w+.*\b{w}\s*[,)]|^\s*(?:@[\w.]+(?:\([^)]*\))?\s+)*[\w.<>\[\]]+\s+{w}\s*(?:,\s*$|\))"
            ))
            .expect("an escaped name keeps the pattern valid");
            hits.retain(|h| {
                call_re.is_match(&h.text) || is_class(h) || component.is_match(&h.text)
            });
        }
        let Some((count, angle)) = arguments(&self.buf.lines, self.line, range.end) else {
            return hits;
        };
        if angle || hits.len() < 2 {
            return hits;
        }
        let map_argument = groovy && constructed && MAP_ARGUMENT.is_match(after);
        let count = if map_argument { 1 } else { count };
        let fits = |h: &Hit| {
            if !java_or_groovy(h) || is_class(h) {
                return true;
            }
            self.text_of(&h.path)
                .and_then(|t| search::jvm_parameters(&t, h.line1, word, search::groovy(&h.path)))
                .is_none_or(|(n, more)| (n..=n.saturating_add(more)).contains(&count))
        };
        let fit: Vec<Hit> = hits.iter().filter(|h| fits(h)).cloned().collect();
        let fit = if map_argument {
            let map = Regex::new(&format!(r"\b{w}\s*\(\s*(?:final\s+)?Map\b"))
                .expect("an escaped name keeps the pattern valid");
            let maps: Vec<Hit> = fit
                .iter()
                .filter(|h| map.is_match(&h.text))
                .cloned()
                .collect();
            if maps.is_empty() { fit } else { maps }
        } else {
            fit
        };
        if fit.is_empty() { hits } else { fit }
    }

    fn jvm_enum_of(&self, here: &Path, chain: &[String]) -> Vec<Hit> {
        let text = self.buf.lines.join("\n");
        let enum_re = Regex::new(r"\benum\s+(?:class\s+)?(\w+)").expect("a fixed pattern");
        let hit = |path: &Path, line: usize, text: &str| Hit {
            deleted: None,
            path: path.to_path_buf(),
            line1: line,
            byte_col: None,
            text: text.to_owned(),
        };
        match chain {
            [] => search::jvm_enclosing_types(&text, self.line + 1)
                .into_iter()
                .find(|&d| enum_re.is_match(&self.buf.lines[d - 1]))
                .map(|d| hit(here, d, &self.buf.lines[d - 1]))
                .into_iter()
                .collect(),
            [owner] => {
                let pattern = search::def_patterns(Kind::Jvm, owner).join("|");
                let mut found = self.project_definitions(Kind::Jvm, here, owner, &pattern);
                found.retain(|h| enum_re.captures(&h.text).is_some_and(|c| &c[1] == owner));
                found
            }
            _ => Vec::new(),
        }
    }

    fn kotlin_infix(before: &str, after: &str, word: &str) -> bool {
        if KOTLIN_KEYWORDS.contains(&word) || after.starts_with('(') || before.ends_with('.') {
            return false;
        }
        let b = before.trim_end();
        let a = after.trim_start();
        if b.len() == before.len() || a.len() == after.len() {
            return false;
        }
        let token = |s: &str| -> String { s.chars().take_while(|c| is_word(*c)).collect() };
        let last: String = {
            let rev: String = b.chars().rev().take_while(|c| is_word(*c)).collect();
            rev.chars().rev().collect()
        };
        let next = token(a);
        let operand_end = b.ends_with([')', ']', '"', '\'']) || !last.is_empty();
        let operand_start = a.starts_with(['(', '"', '\'']) || !next.is_empty();
        let keyword = |t: &str| KOTLIN_KEYWORDS.contains(&t);
        let condition = b.ends_with(')') && {
            let mut depth = 0i32;
            let open = b
                .char_indices()
                .rev()
                .find(|&(_, c)| {
                    match c {
                        ')' => depth += 1,
                        '(' => depth -= 1,
                        _ => {}
                    }
                    depth == 0
                })
                .map_or(0, |(i, _)| i);
            let head: String = {
                let t = b.split_at(open).0.trim_end();
                let rev: String = t.chars().rev().take_while(|c| is_word(*c)).collect();
                rev.chars().rev().collect()
            };
            matches!(head.as_str(), "if" | "while" | "for" | "when" | "catch")
        };
        operand_end
            && operand_start
            && !condition
            && !keyword(&last)
            && !keyword(&next)
            && !b.ends_with(['=', ':', ',', '{', '('])
    }
}

pub(super) fn declared_as(kind: Kind, word: &str, text: &str) -> String {
    if kind != Kind::Jvm || whole_at(text, word, "").is_some() {
        return word.to_owned();
    }
    search::jvm_accessor(word)
        .and_then(|a| (a.fields_it_may_read.into_iter()).find(|n| whole_at(text, n, "").is_some()))
        .unwrap_or_else(|| word.to_owned())
}

impl App {
    pub(super) fn scala_enum_case(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        found: &mut Vec<Candidate>,
    ) {
        if found.is_empty() {
            *found = self.scala_imported_case(here, text, word);
            return;
        }
        let Some(enums) = found
            .iter()
            .map(|c| self.scala_enum_of_case(&c.hit, word))
            .collect::<Option<Vec<(PathBuf, usize)>>>()
        else {
            return;
        };
        let imports = search::jvm_enum_imports(here, text, word);
        let single = imports.iter().any(|i| i.single);
        let Some(imported) = imports
            .iter()
            .filter(|i| i.single == single)
            .map(|i| match self.jvm_type_at(here, text, &i.owner) {
                super::jvm_typed::JvmType::Project(decl) => Some((decl.path, decl.line1)),
                _ => None,
            })
            .collect::<Option<Vec<(PathBuf, usize)>>>()
        else {
            return;
        };
        let keep: Vec<bool> = enums.iter().map(|e| imported.contains(e)).collect();
        if keep.contains(&true) {
            let mut keep = keep.into_iter();
            found.retain(|_| keep.next().unwrap_or_default());
        }
    }

    fn scala_imported_case(&self, here: &Path, text: &str, word: &str) -> Vec<Candidate> {
        let mut out = Vec::new();
        for import in search::jvm_enum_imports(here, text, word) {
            let super::jvm_typed::JvmType::Project(decl) =
                self.jvm_type_at(here, text, &import.owner)
            else {
                continue;
            };
            let Some(body) = self.text_of(&decl.path).filter(|_| import.single) else {
                continue;
            };
            let target = Some((decl.path.clone(), decl.line1));
            out.extend(body.lines().enumerate().filter_map(|(i, l)| {
                let hit = Hit {
                    deleted: None,
                    path: decl.path.clone(),
                    line1: i + 1,
                    byte_col: None,
                    text: l.to_owned(),
                };
                (self.scala_enum_of_case(&hit, word) == target).then(|| Candidate {
                    hit,
                    reason: Reason::Path(import.owner.clone()),
                })
            }));
        }
        out
    }

    fn scala_enum_of_case(&self, hit: &Hit, word: &str) -> Option<(PathBuf, usize)> {
        static ENUM: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"\benum\s+[A-Za-z_]\w*").expect("the regex is a literal")
        });
        let case = Regex::new(&format!(
            r"^\s*case\s+(?:\w+\s*,\s*)*{}\b",
            regex::escape(word)
        ))
        .ok()?;
        if !search::scala(&hit.path) || !case.is_match(&hit.text) {
            return None;
        }
        let text = self.text_of(&hit.path)?;
        let &decl = search::jvm_enclosing_types(&text, hit.line1).first()?;
        ENUM.is_match(text.lines().nth(decl - 1)?)
            .then(|| (hit.path.clone(), decl))
    }
}

impl App {
    pub(super) fn jvm_written_package(
        &self,
        file: &Path,
        text: &str,
        written: &str,
        on: Option<usize>,
    ) -> Option<super::jvm_typed::JvmType> {
        let (head, rest) = written.split_once('.')?;
        if !head.starts_with(|c: char| c.is_ascii_lowercase()) {
            return None;
        }
        let packages = self.jvm_packages()?;
        let aliased = search::scala(file).then(|| {
            let imports = on.map_or_else(
                || search::jvm_imports(file, text),
                |line1| search::jvm_imports_at(file, text, line1),
            );
            let import = imports.into_iter().find(|i| i.bound_name == head)?;
            let path: Vec<String> = (import.path.into_iter())
                .chain(rest.split('.').map(str::to_owned))
                .collect();
            self.jvm_in_package(&packages, &path)
        });
        aliased.flatten().or_else(|| {
            let path: Vec<String> = written.split('.').map(str::to_owned).collect();
            self.jvm_in_package(&packages, &path)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_are_counted_past_non_ascii_characters() {
        let lines = |ls: &[&str]| ls.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(
            arguments(&lines(&["naïve = Straße(ñ, ü)"]), 0, 16),
            Some((2, false))
        );
        assert_eq!(
            arguments(&lines(&[r#"f("ñ", 'ü')"#]), 0, 1),
            Some((2, false))
        );
        assert_eq!(
            arguments(
                &lines(&["\tx := new(", "s = \"open", "t = ü + `x`", ")"]),
                0,
                9
            ),
            Some((1, false))
        );
        assert_eq!(
            arguments(&lines(&["f(\"\"\"", "  text", "  \"\"\")"]), 0, 1),
            Some((1, false))
        );
    }
}
