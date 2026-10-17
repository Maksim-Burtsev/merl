use super::{App, Candidate, HashSet, Hit, Kind, Path, Reason, search};

impl App {
    pub(super) fn concern_class_methods(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        hits: Vec<Hit>,
    ) -> Vec<Hit> {
        let (Some(owner), path) = (chain.last(), chain.join(".")) else {
            return Vec::new();
        };
        let pattern = search::def_patterns(kind, owner).join("|");
        let cut = self.truncated.get();
        let declared = self.project_definitions(kind, here, owner, &pattern);
        self.truncated.set(cut);
        let mut modules: Vec<String> = Vec::new();
        for h in declared {
            let Some(text) = self.text_of(&h.path) else {
                continue;
            };
            let q = search::qualified(kind, &text, h.line1, owner).unwrap_or_else(|| owner.clone());
            if q != path && !q.ends_with(&format!(".{path}")) {
                continue;
            }
            let includes = search::ruby_class_parents(&text, h.line1).includes;
            modules.extend(
                includes
                    .iter()
                    .filter_map(|m| m.rsplit("::").next().map(str::to_owned)),
            );
        }
        hits.into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    modules
                        .iter()
                        .any(|m| search::ruby_concern_class_method(&t, h.line1, m))
                })
            })
            .collect()
    }

    pub(super) fn ruby_class_method_elsewhere(
        &mut self,
        here: &Path,
        chain: &[String],
        word: &str,
        pattern: &str,
        hits: Option<Vec<Hit>>,
    ) -> (String, Vec<Candidate>) {
        let path = chain.join(".");
        let mut found: Vec<Candidate> = hits
            .map(|hits| self.concern_class_methods(Kind::Ruby, here, chain, hits))
            .unwrap_or_default()
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Path(path.clone()),
            })
            .collect();
        if found.is_empty() && word != "initialize" {
            found = self.ruby_extended(here, chain, word);
        }
        match (found.is_empty(), word == "initialize") {
            (true, true) => ("new".to_owned(), found),
            (true, false) => (word.to_owned(), self.ruby_outside(word, pattern)),
            (false, _) => (word.to_owned(), found),
        }
    }

    pub(super) fn ruby_member_outside(
        &mut self,
        word: &str,
        pattern: &str,
        found: &mut Vec<Candidate>,
    ) {
        self.offer_only = true;
        for c in self.ruby_outside(word, pattern) {
            if !found
                .iter()
                .any(|f| f.hit.path == c.hit.path && f.hit.line1 == c.hit.line1)
            {
                found.push(c);
            }
        }
    }

    fn ruby_outside(&mut self, word: &str, pattern: &str) -> Vec<Candidate> {
        let files = self.external_files(Kind::Ruby);
        let hits = self.external_grep(Kind::Ruby, &files, pattern);
        self.declaring(Kind::Ruby, word, hits)
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    pub(super) fn ruby_self_methods(&self, here: &Path, text: &str, word: &str) -> Vec<Candidate> {
        let class = search::ruby_class_path(text, self.line + 1);
        if class.is_empty() {
            return Vec::new();
        }
        let on_class = search::ruby_self_is_class(text, self.line + 1);
        self.ruby_methods_from(here, word, class, on_class)
    }

    fn ruby_extended(&self, here: &Path, chain: &[String], word: &str) -> Vec<Candidate> {
        self.ruby_class_named(here, &chain.join("::"))
            .map(|class| self.ruby_methods_from(here, word, class, true))
            .unwrap_or_default()
    }

    pub(super) fn ruby_constructed(&self, here: &Path, made: &str, word: &str) -> Vec<Candidate> {
        let Some(class) = self.ruby_class_named(here, made) else {
            return Vec::new();
        };
        if !self
            .ruby_methods_from(here, "new", class.clone(), true)
            .is_empty()
        {
            return Vec::new();
        }
        self.ruby_methods_from(here, word, class, false)
    }

    fn ruby_class_named(&self, here: &Path, name: &str) -> Option<String> {
        let from = search::ruby_class_path(&self.text_of(here)?, self.line + 1);
        let own = from == name || from.ends_with(&format!("::{name}"));
        self.ruby_resolve(here, &from, name)
            .or_else(|| own.then_some(from))
    }

    fn ruby_methods_from(
        &self,
        here: &Path,
        word: &str,
        class: String,
        on_class: bool,
    ) -> Vec<Candidate> {
        let mut patterns = search::def_patterns(Kind::Ruby, word);
        let assignment = search::ruby_assignment(word);
        patterns.retain(|p| *p != assignment);
        let owned: Vec<Owned> = self
            .project_definitions(Kind::Ruby, here, word, &patterns.join("|"))
            .into_iter()
            .filter_map(|h| {
                let t = self.text_of(&h.path)?;
                Some(Owned {
                    class: search::ruby_class_path(&t, h.line1),
                    on_class: search::ruby_on_class(&t, h.line1),
                    module_method_on_both_sides: search::ruby_singleton(&t, h.line1),
                    hit: h,
                })
            })
            .collect();
        let mut seen = HashSet::from([class.clone()]);
        let mut queue = std::collections::VecDeque::from([(class, on_class)]);
        while let Some((c, side)) = queue.pop_front() {
            let found: Vec<Candidate> = owned
                .iter()
                .filter(|o| {
                    o.class == c
                        && if side {
                            o.module_method_on_both_sides
                        } else {
                            !o.on_class
                        }
                })
                .map(|o| Candidate {
                    hit: o.hit.clone(),
                    reason: Reason::Receiver(c.clone()),
                })
                .collect();
            if !found.is_empty() {
                return found;
            }
            let search::RubyParents {
                superclasses: supers,
                includes,
                extends,
            } = self.ruby_parents(here, &c);
            let mixed = if side { extends } else { includes };
            let next = mixed
                .into_iter()
                .map(|m| (m, false))
                .chain(supers.into_iter().map(|s| (s, side)));
            for (name, side) in next {
                if let Some(path) = self.ruby_resolve(here, &c, &name)
                    && seen.insert(path.clone())
                {
                    queue.push_back((path, side));
                }
            }
        }
        Vec::new()
    }

    fn ruby_parents(&self, here: &Path, path: &str) -> search::RubyParents {
        let mut out = search::RubyParents::default();
        for (h, t) in self.ruby_declarations(here, path) {
            let one = search::ruby_class_parents(&t, h.line1);
            out.superclasses.extend(one.superclasses);
            out.includes.extend(one.includes);
            out.extends.extend(one.extends);
        }
        out
    }

    fn ruby_declarations(&self, here: &Path, path: &str) -> Vec<(Hit, String)> {
        let last = path.rsplit("::").next().unwrap_or(path);
        let pattern = format!(
            r"^\s*(?:class|module)\s+(?:[\w:]+::)?{}(?:[^\w:]|$)",
            regex::escape(last)
        );
        self.project_definitions(Kind::Ruby, here, last, &pattern)
            .into_iter()
            .filter_map(|h| {
                let t = self.text_of(&h.path)?;
                (search::ruby_declared_path(&t, h.line1).as_deref() == Some(path)).then_some((h, t))
            })
            .collect()
    }

    fn ruby_resolve(&self, here: &Path, class: &str, name: &str) -> Option<String> {
        let (name, mut scope) = match name.strip_prefix("::") {
            Some(top) => (top, ""),
            None => (name, class),
        };
        let last = name.rsplit("::").next().unwrap_or(name);
        let pattern = format!(
            r"^\s*(?:class|module)\s+(?:[\w:]+::)?{}(?:[^\w:]|$)",
            regex::escape(last)
        );
        let paths: HashSet<String> = self
            .project_definitions(Kind::Ruby, here, last, &pattern)
            .into_iter()
            .filter_map(|h| search::ruby_declared_path(&self.text_of(&h.path)?, h.line1))
            .collect();
        loop {
            let want = if scope.is_empty() {
                name.to_owned()
            } else {
                format!("{scope}::{name}")
            };
            if want != class && paths.contains(&want) {
                return Some(want);
            }
            if scope.is_empty() {
                return None;
            }
            scope = scope.rsplit_once("::").map_or("", |(outer, _)| outer);
        }
    }

    pub(super) fn ruby_ivars(&self, here: &Path, text: &str, ivar: &str) -> Vec<Candidate> {
        let class = search::ruby_class_path(text, self.line + 1);
        let pattern = search::ruby_assignment(ivar);
        let assigned: Vec<(Hit, String)> = self
            .project_definitions(Kind::Ruby, here, ivar, &pattern)
            .into_iter()
            .filter(|h| !class.is_empty() || h.path == here)
            .filter_map(|h| {
                let c = search::ruby_class_path(&self.text_of(&h.path)?, h.line1);
                Some((h, c))
            })
            .collect();
        if class.is_empty() {
            return assigned
                .into_iter()
                .filter(|(_, c)| c.is_empty())
                .map(|(hit, _)| Candidate {
                    hit,
                    reason: Reason::File,
                })
                .collect();
        }
        let mut seen = HashSet::from([class.clone()]);
        let mut queue = std::collections::VecDeque::from([class]);
        let mut unread = false;
        while let Some(c) = queue.pop_front() {
            let found: Vec<Candidate> = assigned
                .iter()
                .filter(|(_, hc)| *hc == c)
                .map(|(h, _)| Candidate {
                    hit: h.clone(),
                    reason: Reason::Receiver(c.clone()),
                })
                .collect();
            if !found.is_empty() {
                return found;
            }
            let search::RubyParents {
                superclasses: supers,
                includes,
                ..
            } = self.ruby_parents(here, &c);
            for name in includes.into_iter().chain(supers) {
                match self.ruby_resolve(here, &c, &name) {
                    Some(path) => {
                        if seen.insert(path.clone()) {
                            queue.push_back(path);
                        }
                    }
                    None => unread = true,
                }
            }
        }
        if unread {
            assigned
                .into_iter()
                .map(|(hit, _)| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

struct Owned {
    hit: Hit,
    class: String,
    on_class: bool,
    module_method_on_both_sides: bool,
}
