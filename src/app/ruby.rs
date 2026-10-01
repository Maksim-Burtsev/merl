//! `d` in Ruby: the methods a `self` answers to and the lines that assign an `@ivar`, over the
//! class, its modules and its superclasses (#365, #383), and the class methods a concern gives
//! (#387).

use super::*;

impl App {
    /// Of `hits`, the declarations of a Ruby method that a concern included by the class
    /// `chain` spells gives it as a class method (#387): `include Searchable` in a file declaring
    /// the class, and the method in `Searchable`'s `class_methods do` or `module ClassMethods`.
    pub(super) fn concern_class_methods(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        hits: Vec<Hit>,
    ) -> Vec<Hit> {
        static INCLUDE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^\s*include\s+([A-Z][\w:]*(?:\s*,\s*[A-Z][\w:]*)*)").unwrap()
        });
        let (Some(owner), path) = (chain.last(), chain.join(".")) else {
            return Vec::new();
        };
        let pattern = search::def_patterns(kind, owner).join("|");
        let cut = self.truncated.get();
        let declared = self.project_definitions(kind, here, owner, &pattern);
        self.truncated.set(cut);
        // ponytail: every `include` of the file, not only the ones inside the class's body.
        let mut modules: Vec<String> = Vec::new();
        for h in declared {
            let Some(text) = self.text_of(&h.path) else {
                continue;
            };
            let q = search::qualified(kind, &text, h.line, owner).unwrap_or_else(|| owner.clone());
            if q != path && !q.ends_with(&format!(".{path}")) {
                continue;
            }
            for c in text.lines().filter_map(|l| INCLUDE.captures(l)) {
                modules.extend(
                    c[1].split(',')
                        .filter_map(|m| m.trim().rsplit("::").next().map(str::to_owned)),
                );
            }
        }
        hits.into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    modules
                        .iter()
                        .any(|m| search::ruby_concern_class_method(&t, h.line, m))
                })
            })
            .collect()
    }

    /// What the Ruby `Const.word` answers when no declaration of the class `chain` spells it
    /// (#387), with the word asked for: a class method a concern the class includes gives it, of
    /// `hits`, the project's declarations of the name; else, by name, what Ruby's core and the
    /// gems declare (#369), ActiveRecord's `find`. `Const.new` with no `initialize` of the class's
    /// own is `Class#new` itself, asked for as `new`.
    pub(super) fn ruby_class_method_elsewhere(
        &mut self,
        here: &Path,
        chain: &[String],
        word: &str,
        pattern: &str,
        hits: Option<Vec<Hit>>,
    ) -> (String, Vec<Candidate>) {
        let path = chain.join(".");
        let found: Vec<Candidate> = hits
            .map(|hits| self.concern_class_methods(Kind::Ruby, here, chain, hits))
            .unwrap_or_default()
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Path(path.clone()),
            })
            .collect();
        match (found.is_empty(), word == "initialize") {
            (true, true) => ("new".to_owned(), found),
            (true, false) => (word.to_owned(), self.ruby_outside(word, pattern)),
            (false, _) => (word.to_owned(), found),
        }
    }

    /// `x.word` on a Ruby value of no known type, `found` the candidates so far: what Ruby's core
    /// and the gems declare of the name joins them (#369), so the project's one namesake is no
    /// longer alone. Whatever is read, one candidate proves nothing of a value whose type is not
    /// known, an ActiveRecord column's reader is declared nowhere: it is offered rather than
    /// jumped to (#390).
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
                .any(|f| f.hit.path == c.hit.path && f.hit.line == c.hit.line)
            {
                found.push(c);
            }
        }
    }

    /// The declarations of `word` that `pattern` finds outside the project, in Ruby's core, its
    /// standard library and the gems, by name (#369).
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

    /// The methods named `word` a Ruby `self` at the cursor answers to (#365): those its class
    /// declares, in any file that opens it, then those of the modules it `include`s (or, when
    /// `self` is the class, `extend`s), then its superclass's, walked up. When `self` is the
    /// class, only the class's own methods count. Empty when none declares it, or at the top of a
    /// file.
    pub(super) fn ruby_self_methods(&self, here: &Path, text: &str, word: &str) -> Vec<Candidate> {
        let class = search::ruby_class_path(text, self.line + 1);
        if class.is_empty() {
            return Vec::new();
        }
        let on_class = search::ruby_self_is_class(text, self.line + 1);
        let mut patterns = search::def_patterns(Kind::Ruby, word);
        let assignment = search::ruby_assignment(word);
        patterns.retain(|p| *p != assignment);
        // Each declaration with its class, whether it is on the class, and whether it is a
        // method of a module every side of which counts.
        let owned: Vec<(Hit, String, bool, bool)> = self
            .project_definitions(Kind::Ruby, here, word, &patterns.join("|"))
            .into_iter()
            .filter_map(|h| {
                let t = self.text_of(&h.path)?;
                let c = search::ruby_class_path(&t, h.line);
                let on = search::ruby_on_class(&t, h.line);
                let both = search::ruby_singleton(&t, h.line);
                Some((h, c, on, both))
            })
            .collect();
        let mut seen = HashSet::from([class.clone()]);
        let mut queue = std::collections::VecDeque::from([(class, on_class)]);
        while let Some((c, side)) = queue.pop_front() {
            let found: Vec<Candidate> = owned
                .iter()
                .filter(|(_, hc, on, both)| *hc == c && if side { *both } else { !*on })
                .map(|(h, ..)| Candidate {
                    hit: h.clone(),
                    reason: Reason::Receiver(c.clone()),
                })
                .collect();
            if !found.is_empty() {
                return found;
            }
            let (supers, includes, extends) = self.ruby_parents(here, &c);
            let mixed = match side {
                true => extends,
                false => includes,
            };
            // A module's methods mixed in are its instance methods, whichever side takes them.
            let next = mixed
                .into_iter()
                .map(|m| (m, false))
                .chain(supers.into_iter().map(|s| (s, side)));
            for (name, side) in next {
                for path in self.ruby_resolve(here, &name) {
                    if seen.insert(path.clone()) {
                        queue.push_back((path, side));
                    }
                }
            }
        }
        Vec::new()
    }

    /// The superclasses, `include`d and `extend`ed modules of the Ruby class or module `path`, as
    /// written, over every file of the project that opens it.
    fn ruby_parents(&self, here: &Path, path: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut out = (Vec::new(), Vec::new(), Vec::new());
        for (h, t) in self.ruby_declarations(here, path) {
            let (s, i, e) = search::ruby_class_parents(&t, h.line);
            out.0.extend(s);
            out.1.extend(i);
            out.2.extend(e);
        }
        out
    }

    /// The lines of the project that open the Ruby class or module `path`, with their file's text.
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
                (search::ruby_declared_path(&t, h.line).as_deref() == Some(path)).then_some((h, t))
            })
            .collect()
    }

    /// The paths of the classes and modules the project declares that the constant `name`, as
    /// written, can name: `Tariff` is `Shop::Tariff` as well.
    /// ponytail: Ruby's lexical lookup is not modelled; every such path counts.
    fn ruby_resolve(&self, here: &Path, name: &str) -> Vec<String> {
        let last = name.rsplit("::").next().unwrap_or(name);
        let pattern = format!(
            r"^\s*(?:class|module)\s+(?:[\w:]+::)?{}(?:[^\w:]|$)",
            regex::escape(last)
        );
        let mut paths: Vec<String> = self
            .project_definitions(Kind::Ruby, here, last, &pattern)
            .into_iter()
            .filter_map(|h| search::ruby_declared_path(&self.text_of(&h.path)?, h.line))
            .filter(|p| p == name || p.ends_with(&format!("::{name}")))
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }

    /// The lines that assign the Ruby instance or class variable `ivar` in the class around the
    /// cursor, in `here` or in another file that reopens the class (#383), else in the modules it
    /// `include`s and its superclasses, walked up as [`Self::ruby_self_methods`] walks them: a
    /// controller's `@current_user` is set by its parent's before-action. When no class of that
    /// chain assigns it and a parent is not the project's (`ActionController::Base`, a gem's
    /// module), every assignment of the project counts, by name. At the top of a file, that
    /// file's own.
    pub(super) fn ruby_ivars(&self, here: &Path, text: &str, ivar: &str) -> Vec<Candidate> {
        let class = search::ruby_class_path(text, self.line + 1);
        let pattern = search::ruby_assignment(ivar);
        let assigned: Vec<(Hit, String)> = self
            .project_definitions(Kind::Ruby, here, ivar, &pattern)
            .into_iter()
            .filter(|h| !class.is_empty() || h.path == here)
            .filter_map(|h| {
                let c = search::ruby_class_path(&self.text_of(&h.path)?, h.line);
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
            let (supers, includes, _) = self.ruby_parents(here, &c);
            for name in includes.into_iter().chain(supers) {
                let paths = self.ruby_resolve(here, &name);
                unread |= paths.is_empty();
                for path in paths {
                    if seen.insert(path.clone()) {
                        queue.push_back(path);
                    }
                }
            }
        }
        match unread {
            true => assigned
                .into_iter()
                .map(|(hit, _)| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect(),
            false => Vec::new(),
        }
    }
}
