//! `d` / F12: the word under the cursor, and where its declaration is looked for.

use super::*;

impl App {
    /// `d` / F12. A file of a known [`Kind`] gets its declaration patterns. A word or a qualifier
    /// an import binds is looked for in the module the import names: the project's file or
    /// package (`from app.repos import X` in `app/repos.py`, `store.Open` in the `store`
    /// directory), else the standard library and the installed dependencies
    /// ([`search::external_roots`]) narrowed to that module (`np.array` in `numpy`). Everything
    /// else, and a project module that does not declare the word, is searched in the project where
    /// such a definition can live (`.tsx` finds `.ts`, a Terraform variable stays in its module),
    /// and nothing there sends the same patterns outside it.
    ///
    /// `x.word` on a value — a `.` qualifier no import binds, other than a bare `self` — is a
    /// member whose type is not known: every member declaration of the name, in the project and
    /// outside it, and every field of the name in the project, is a candidate. The status line
    /// says how the target was found, and a single candidate found only by name says so too. An
    /// enum variant or a parameter has no declaration the rules know and gets "no definition":
    /// `u` lists the uses.
    pub(super) fn goto_definition(&mut self) {
        let kind = self.kind();
        let Some((range, word)) = search::definition_word(kind, self.line_str(), self.col) else {
            self.message = "no word".into();
            return;
        };
        let word = word.to_owned();
        // A member access broken over lines reads as the one line it is.
        let (written, start) = kind
            .and_then(|k| search::unbroken(k, &self.buf.lines, self.line, range.start))
            .unwrap_or_else(|| (self.line_str().to_owned(), range.start));
        let (written, start) = match kind {
            Some(k) => search::plain_access(k, &written, start),
            None => (written, start),
        };
        let before = &written[..start];
        let dotted = before.ends_with('.') && !before.ends_with("..");
        let chain = search::qualifier(&written, start);
        let here = self.rel_current();
        let (Some(kind), Some(here)) = (kind, here) else {
            self.message = self.no_rules();
            return;
        };
        let text = self.buf.lines.join("\n");
        self.offer_only =
            kind == Kind::Python && search::keyword_argument(&text, self.line + 1, &range);
        self.truncated.set(false);
        // Inside a docstring's example the imports written there count too.
        let in_literal = search::literal_lines(kind, &text).get(self.line) == Some(&true);
        let mut imports = match in_literal {
            true => search::imports_as_written(kind, &text),
            false => search::imports(kind, &text),
        };
        // A parameter or a local of the same name hides the import where the cursor is: `json`
        // in `def handler(json)` is a value, and `via import` would be a proof of nothing.
        let first = chain.first().map_or(word.as_str(), String::as_str);
        // `super` is no local, whatever the member lookup reads it as.
        let locals: Vec<usize> = search::bindings(kind, &text, self.line + 1, first)
            .iter()
            .map(|b| b.line)
            .filter(|&n| {
                (dotted || word != "super") && !names_itself(&self.buf.lines[n - 1], first)
            })
            .collect();
        if !locals.is_empty() {
            imports.retain(|(name, _)| name != first);
        }
        // The word itself is that parameter or local: its declarations in this scope are the
        // answer, and a function of the same name elsewhere is not.
        if !dotted && !locals.is_empty() && locals != [self.line + 1] {
            let found = locals
                .iter()
                .map(|&line| Candidate {
                    hit: Hit {
                        path: here.clone(),
                        line,
                        text: self.buf.lines[line - 1].clone(),
                    },
                    reason: Reason::Local,
                })
                .collect();
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A Go package qualifier, `db` in `db.Get`, is declared by the import line of this file
        // (#100), unless a local hides it (taken out above, or one the walk may have missed:
        // the function mentions the name other than as a qualifier) or the package declares the
        // name itself: an import's name is read off its path, and a package may be called
        // otherwise.
        if kind == Kind::Go
            && !dotted
            && self.line_str()[range.end..].starts_with('.')
            && let Some(path) = bound(&imports, &word)
            && let Some(line) = search::go_import_line(&text, &word)
            && !search::go_may_declare(&text, self.line + 1, &word)
            && self
                .package_declarations(kind, &here, &word, None)
                .is_empty()
        {
            let hit = Hit {
                path: here.clone(),
                line,
                text: self.buf.lines[line - 1].clone(),
            };
            let reason = Reason::Import(path.join("/"));
            self.show_definitions(kind, &word, &here, vec![Candidate { hit, reason }], None);
            return;
        }
        let own = matches!(chain.as_slice(), [s] if s == "self" || s == "cls" || s == "this");
        let on_value = dotted && !own && chain.first().is_none_or(|f| bound(&imports, f).is_none());
        let members = on_value
            .then(|| search::member_patterns(kind, &word))
            .flatten()
            .map(|m| m.join("|"));
        let patterns = search::def_patterns(kind, &word);
        if patterns.is_empty() {
            self.message = self.no_rules();
            return;
        }
        let pattern = patterns.join("|");
        // On the declaration of a member of an interface, a protocol, an abstract or a base
        // class, `d` offers what implements it (#68, step 6).
        // A `#private` member is nobody's to override.
        if !dotted && !word.starts_with('#') {
            let found = self.implementations(kind, &here, &text, &word);
            if !found.is_empty() {
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
        }
        // On the name a field's declaration gives it, the other fields and members of the name
        // are namesakes (#104). `self.repo = repo` is decided below, by where `self.repo` leads.
        if !dotted && search::field_decl_at(kind, &text, self.line + 1, range.start, &word) {
            let found = self.field_namesakes(kind, &here, &word);
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A receiver whose type is proven narrows the member to that type (#68, steps 2 to 4).
        let mut broke = None;
        // A chain with no name to start from may hang off a call: `make_uow().users.word` (#100).
        let head = (dotted && chain.is_empty())
            .then(|| search::call_head(kind, &written, start))
            .flatten();
        if dotted
            && matches!(kind, Kind::Python | Kind::TsJs | Kind::Go)
            && (head.is_some() || chain.first().is_some_and(|f| bound(&imports, f).is_none()))
        {
            match self.typed_definitions(kind, &here, &word, &chain, head.as_ref()) {
                // `self.repo` that leads to this very line is the field's declaration.
                Ok(found)
                    if own
                        && matches!(found.as_slice(), [c] if c.hit.line == self.line + 1 && c.hit.path == here) =>
                {
                    let found = self.field_namesakes(kind, &here, &word);
                    self.show_definitions(kind, &word, &here, found, None);
                    return;
                }
                Ok(found) if !found.is_empty() => {
                    self.show_definitions(kind, &word, &here, found, None);
                    return;
                }
                Ok(_) => {}
                // With one name in front of the word, `by name` already says where.
                Err(at) => {
                    let names = head.as_ref().map_or(chain.len(), |(_, _, f)| f.len() + 1);
                    broke = (names > 1).then_some(at);
                }
            }
        }
        // An import names where the word is declared: the project's module, else the one outside
        // it. Only a module of the project that does not declare it (a re-export) leaves the word
        // to the search by name.
        let import = (!on_value)
            .then(|| {
                bound(
                    &imports,
                    chain.first().map_or(word.as_str(), String::as_str),
                )
            })
            .flatten();
        let mut outside = false;
        // The import names a module of the project's own: a workspace package linked in, an alias.
        let mut own_module = false;
        let mut found = match import {
            Some(path) => {
                let mut found = self
                    .imported_definitions(kind, &here, &word, &chain, &path)
                    .unwrap_or_else(|| {
                        // A workspace package linked in is the project's own: the search by
                        // name in the project comes first, the one outside after it (below).
                        let found =
                            self.external_definitions(kind, &word, &chain, dotted, &imports, true);
                        outside = found.is_some();
                        own_module = found.is_none();
                        found.unwrap_or_default()
                    });
                // `try: from a import pick` / `except ImportError: from b import pick` names
                // two sources: both are offered, and which one ran is not for `d` to guess.
                let others: Vec<Vec<String>> = imports
                    .iter()
                    .filter(|(name, other)| name == first && *other != path)
                    .map(|(_, other)| other.clone())
                    .collect();
                for other in others {
                    let more = self
                        .imported_definitions(kind, &here, &word, &chain, &other)
                        .unwrap_or_default();
                    found.extend(more);
                }
                found
            }
            None => Vec::new(),
        };
        if !found.is_empty() {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // Python's `Cls.CONST`, an `Enum` member, a dataclass field (#100): the qualifier is a
        // class the file declares or imports, no value of the scope, and the word is what the
        // class body declares, or a class above it.
        // A class or an import inside a function is not the one the top of the file names.
        let nested = |a: &Self| {
            search::bindings(kind, &text, a.line + 1, first)
                .iter()
                .any(|b| a.buf.lines[b.line - 1].starts_with([' ', '\t']))
        };
        if kind == Kind::Python && locals.is_empty() && !chain.is_empty() && !nested(self) {
            let found = self.class_attribute(kind, &here, &chain, &word);
            if !found.is_empty() {
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
        }
        // A member in the project first. A qualifier no import names can still be a class, a
        // namespace or a module of the project, which declares the word at its top level.
        // A qualifier that is no value and no import can be what declares the word: a namespace,
        // a class with a static member, a nested class. A declaration that reads `Outer.find`
        // is the answer then, and a method `find` of some other class is not.
        // The chain is joined as the kind qualifies a name (#129): Rust and C++ write the path
        // with `::`, where no `.` stands in front of the word, and a `.` there is a value's.
        let sep = search::separator(kind);
        let path = chain.join(sep);
        let pathed = match sep {
            "." => on_value,
            _ => self.line_str()[..range.start].ends_with(&format!("{path}{sep}")),
        };
        // A type's name is no proof of which type (#129): in a `::` kind the project has to
        // declare it once, and a `use` of the file must not bind the path's first name to
        // somewhere outside, whatever `io.rs` the project has beside `std::io`. A Rust type the
        // project declares more than once is proven by the file of the module a `use crate::…`
        // takes it from (#227): an `impl` at the top of that file is for the type the path
        // names, declared there or handed on, and nothing else is offered then.
        let mut home: Option<Vec<PathBuf>> = None;
        let pathed = pathed
            && !chain.is_empty()
            && (sep == "." || {
                // A glob, a `using` or a grouped `use` may bring the name in unread, and a PHP
                // `\Vendor\Depot::open` spells out another namespace.
                let owner = &chain[chain.len() - 1];
                let brings = Regex::new(&format!(
                    r"^\s*(?:pub\s+)?us(?:e|ing)\b.*(?:\*|\bnamespace\b|\b{}\b)",
                    regex::escape(owner)
                ))
                .expect("an escaped name keeps the pattern valid");
                let inside = |l: &str| ["crate", "self::", "super::"].iter().any(|k| l.contains(k));
                let outside = bound(&imports, &chain[0])
                    .is_some_and(|p| !matches!(p[0].as_str(), "crate" | "self" | "super"))
                    || self
                        .buf
                        .lines
                        .iter()
                        .any(|l| brings.is_match(l) && !inside(l))
                    || self.line_str()[..range.start]
                        .strip_suffix(&format!("{path}{sep}"))
                        .is_some_and(|b| b.ends_with('\\'));
                let owners = search::def_patterns(kind, owner).join("|");
                let declared = self.project_definitions(kind, &here, owner, &owners);
                // A type of the same name in this file is the one its own scope sees.
                if !outside
                    && declared.len() > 1
                    && kind == Kind::Rust
                    && chain.len() == 1
                    && !declared.iter().any(|h| h.path == here)
                {
                    let files = search::rust_use_files(&self.files, &here, &text, owner);
                    home = (!files.is_empty()).then_some(files);
                }
                // A cut in this grep says nothing about the list shown in the end.
                self.truncated.set(false);
                !outside && (declared.len() == 1 || home.is_some())
            });
        if pathed && locals.is_empty() && !chain.is_empty() {
            let full = format!("{path}{sep}{word}");
            // `depot::Shed::open` has the modules of the project in front of `Shed::open`.
            let in_project = sep != "." && self.names_module(&chain[0]);
            // Only the files of the module a `use` names can hold the answer then.
            let hits = match &home {
                Some(files) => self
                    .grep(&pattern, false, false, |p| files.iter().any(|f| f == p))
                    .unwrap_or_default(),
                None => self.project_definitions(kind, &here, &word, &pattern),
            };
            let named: Vec<Candidate> = hits
                .into_iter()
                .filter(|h| {
                    self.text_of(&h.path)
                        .and_then(|t| search::qualified(kind, &t, h.line, &word))
                        .is_some_and(|q| match &home {
                            Some(files) => q == full && files.contains(&h.path),
                            None => {
                                q == full
                                    || q.ends_with(&format!("{sep}{full}"))
                                    || (in_project && full.ends_with(&format!("{sep}{q}")))
                            }
                        })
                })
                .map(|hit| Candidate {
                    reason: match home {
                        Some(_) => Reason::Import(hit.path.display().to_string()),
                        None => Reason::Path(path.clone()),
                    },
                    hit,
                })
                .collect();
            if !named.is_empty() {
                self.show_definitions(kind, &word, &here, named, None);
                return;
            }
            // A cut in a grep whose result is dropped says nothing about the list below.
            self.truncated.set(false);
        }
        // A parameter or a local in front of the word is a value for certain: it has members,
        // and a function or a variable at the top of a module is not one of them.
        let hits = members
            .as_ref()
            .map(|m| self.members_by_name(kind, &here, &word, m))
            .filter(|hits| !hits.is_empty() || !locals.is_empty())
            .unwrap_or_else(|| {
                if own {
                    // `self.word` whose class is not read to the end: the declarations of the name,
                    // and the fields too (#104).
                    self.members_by_name(kind, &here, &word, &pattern)
                } else {
                    self.project_definitions(kind, &here, &word, &pattern)
                }
            });
        found = hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect();
        if let Some(members) = &members {
            // A value's type is unknown: its member may come from a dependency as well. Outside
            // the project TypeScript is read from its declaration files, as VS Code lands on
            // them: the `.d.ts` says what a type offers, the bundled JavaScript is noise.
            let all = self.external_files(kind);
            let files: Vec<PathBuf> = all
                .iter()
                .filter(|p| kind != Kind::TsJs || search::declaration_file(p))
                .cloned()
                .collect();
            let external = self.external_grep(kind, &files, members);
            let seen: Vec<PathBuf> = found.iter().map(|c| self.root.join(&c.hit.path)).collect();
            found.extend(
                external
                    .into_iter()
                    .filter(|h| !seen.contains(&h.path))
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::ByName,
                    }),
            );
        } else if found.is_empty()
            && !outside
            && !matches!(
                chain.first().map(String::as_str),
                Some("self" | "cls" | "this")
            )
        {
            // After the project, outside as the import names the module, every installed copy
            // of it: what a workspace package linked in hands on from a dependency is there.
            found = self
                .external_definitions(kind, &word, &chain, dotted, &imports, false)
                .unwrap_or_default();
            // The module the import loads is the project's, searched already: nothing outside is
            // proven to be what it hands on.
            if own_module {
                for c in &mut found {
                    c.reason = Reason::ByName;
                }
            }
        }
        self.show_definitions(kind, &word, &here, found, broke.as_deref());
    }

    /// Whether `first` starts a path inside the project: `crate`, `self`, `super`, or a file or a
    /// directory of the project called so.
    fn names_module(&self, first: &str) -> bool {
        matches!(first, "crate" | "self" | "super")
            || self.files.iter().any(|f| {
                f.file_stem().is_some_and(|s| s == first)
                    || f.parent().is_some_and(|d| d.ends_with(first))
            })
    }

    /// Jumps to the one candidate, or opens the picker over several, and says how they were found
    /// and at which name of the chain in front of the word the typed lookup `broke`, if it did.
    fn show_definitions(
        &mut self,
        kind: Kind,
        word: &str,
        here: &Path,
        mut found: Vec<Candidate>,
        broke: Option<&str>,
    ) {
        // Standing on one of the definitions is not a reason to go nowhere. But what is left are
        // namesakes nothing ties to this one, so they are offered, never jumped to: a second `d`
        // after a proven jump would walk out of the type it has just proven (#68).
        // A line inside a raw string, a docstring or a block comment declares nothing. Past a
        // few hundred candidates the picker is a list to filter, and reading every file is not
        // worth what it would drop.
        if found.len() <= 500 {
            let mut literal: HashMap<PathBuf, Vec<bool>> = HashMap::new();
            found.retain(|c| {
                let lines = literal.entry(c.hit.path.clone()).or_insert_with(|| {
                    self.text_of(&c.hit.path)
                        .map_or_else(Vec::new, |t| search::literal_lines(kind, &t))
                });
                // `register<` over its type arguments over `>(1);` is a call prettier wrapped.
                let call = kind == Kind::TsJs
                    && c.hit.text.trim_end().ends_with('<')
                    && self
                        .text_of(&c.hit.path)
                        .is_some_and(|t| !search::declares_wrapped_generic(&t, c.hit.line));
                !call && !lines.get(c.hit.line - 1).copied().unwrap_or(false)
            });
        }
        let all = found.len();
        if all > 1 {
            found.retain(|c| c.hit.line != self.line + 1 || c.hit.path != here);
        }
        // The line of an interface method is a declaration no pattern of `d` lists, so nothing
        // was dropped above, and what is found is its namesakes all the same.
        let offer_only = std::mem::take(&mut self.offer_only);
        let truncated = self.truncated.take();
        let on_member = || {
            let at = search::definition_word(Some(kind), self.line_str(), self.col);
            let bare =
                at.is_some_and(|(r, w)| w == word && !self.line_str()[..r.start].ends_with('.'));
            bare && search::member_or_signature(kind, word)
                .and_then(|p| Regex::new(&p.join("|")).ok())
                .is_some_and(|re| re.is_match(self.line_str()))
                && search::owner_decl(kind, &self.buf.lines.join("\n"), self.line + 1).is_some()
        };
        let namesakes = found.iter().all(|c| !c.reason.proven())
            && !found.is_empty()
            && (found.len() < all || on_member())
            // Alone, the declaration under the cursor is its own answer.
            && found
                .iter()
                .any(|c| c.hit.line != self.line + 1 || c.hit.path != here);
        // Tests, mocks, fixtures, generated and vendored copies of a declaration come last here
        // too (#81) — except in the file on screen, which is what the reader is reading. The sort
        // is stable and every candidate is a declaration, so the rest keep the order the search
        // found them in, standard library and all.
        found.sort_by_cached_key(|c| search::rank(&c.hit.path, Some(here), true).0);
        // The project and the outside are each cut at MAX_HITS; the picker holds that many.
        found.truncate(search::MAX_HITS);
        match found.as_slice() {
            [] => self.message = resolution(word, None, &found, broke, truncated),
            [one] if !namesakes && !offer_only && !truncated => {
                let path = self.root.join(&one.hit.path);
                let target = self
                    .text_of(&one.hit.path)
                    .and_then(|text| search::qualified(kind, &text, one.hit.line, word));
                let status = resolution(word, target.as_deref(), &found, broke, false);
                self.jump_to(&path, one.hit.line);
                // A refused jump (edits that cannot be saved) leaves its own reason, not a
                // resolution nobody followed.
                if self.buf.path.as_deref() == Some(path.as_path()) {
                    // The cursor lands on the word rather than at the start of the line, so a
                    // second `d` there asks the next question about the same name: what
                    // implements the declaration it just landed on (#68, step 6).
                    let text = self.line_str().to_owned();
                    let whole = |(i, _): &(usize, &str)| {
                        !text[..*i].ends_with(is_word)
                            && !text[i + word.len()..].starts_with(is_word)
                    };
                    if let Some((i, _)) = text.match_indices(word).find(whole) {
                        self.col = i;
                        self.sync_want_x();
                    }
                    self.message = status;
                }
            }
            _ => {
                let status = if namesakes {
                    let others = if found.len() == 1 { "other" } else { "others" };
                    format!("{word}: at a declaration, {} {others} by name", found.len())
                } else {
                    resolution(word, None, &found, broke, truncated)
                };
                let items = self.definition_items(kind, word, found);
                self.show_picker(PickerKind::Definitions, items);
                if let Some(p) = &mut self.picker {
                    p.title = p
                        .title
                        .replacen(PickerKind::Definitions.title(), &status, 1);
                }
                self.message = status;
            }
        }
    }

    /// The lines `pattern` matches where a definition of `word` in `here`, a file of `kind`, can
    /// live in the project. Escaped or built-in patterns always compile.
    pub(super) fn project_definitions(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        pattern: &str,
    ) -> Vec<Hit> {
        let mut hits = self
            .grep(pattern, false, false, |p| {
                search::in_def_scope(kind, here, p)
            })
            .unwrap_or_default();
        self.note_cut(&hits);
        if let Some(block) = search::def_block(kind, word) {
            hits.retain(|h| {
                std::fs::read_to_string(self.root.join(&h.path))
                    .is_ok_and(|text| search::directly_inside(&text, h.line, block))
            });
        }
        hits
    }

    /// The declarations of `word` in the project module an import names, where `path` is what
    /// [`search::imports`] binds the word or its qualifier to; `None` when the module is not the
    /// project's. The word sits directly in the module, or in the class the chain goes through
    /// (`UserRepo.create`), as [`search::qualified`] names it: `store.Open` is never a method
    /// `Open`. An alias finds the imported name. A default import finds a declaration of its local
    /// name, else the module's `export default`. An empty list is a module that does not declare
    /// the word, a re-export: the search by name takes over.
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
        // The names from the module down to the word: what the import takes, then the chain.
        let tail = |mut names: Vec<String>| {
            if let Some((_, after)) = chain.split_first() {
                names.extend(after.iter().cloned());
                names.push(word.to_owned());
            }
            names
        };
        let (files, inside) = match kind {
            // `from a import b` takes a module or a name from `a`: the longest module that exists,
            // never shorter than the one the import names.
            Kind::Python => {
                let target = tail(path.to_vec());
                let floor = path.len().saturating_sub(1).max(1);
                (floor..target.len()).rev().find_map(|n| {
                    let files = module_files(&target[..n]);
                    (!files.is_empty()).then(|| (files, target[n..].to_vec()))
                })?
            }
            Kind::TsJs => {
                let (taken, module) = path.split_last()?;
                let files = module_files(module);
                if files.is_empty() {
                    return None;
                }
                let inside = match taken.as_str() {
                    "*" => tail(Vec::new()),
                    // What a default export is called is known only on its own line.
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
        let pattern = search::def_patterns(kind, name).join("|");
        let mut hits = self
            .grep(&pattern, false, false, wanted)
            .unwrap_or_default();
        let within = (inside.len() > 1).then(|| inside.join("."));
        hits.retain(|h| {
            self.text_of(&h.path)
                .and_then(|text| search::qualified(kind, &text, h.line, name))
                == within
        });
        // `export { Hono as HonoBase }`: the module declares it under another name. A default
        // import's name is the importer's own.
        let named = path.last().is_some_and(|t| t != "default");
        if hits.is_empty() && kind == Kind::TsJs && within.is_none() && named {
            hits = self.renamed_export(name, |p| {
                self.grep(p, false, false, wanted).unwrap_or_default()
            });
        }
        if hits.is_empty() && kind == Kind::TsJs && path.last().is_some_and(|t| t == "default") {
            hits = self
                .grep(r"^export\s+default\b", false, false, wanted)
                .unwrap_or_default();
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
                    reason: Reason::Import(label(&hit)),
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
}

/// What the status line says after `d` on `word`: `word → Target.word (reason)` for a jump,
/// `word: reason` when the target is not declared inside anything, `word: reason, N
/// declarations` over a picker. A jump by name says it had one match, so a guess that happened to
/// be unique never reads as a resolution. A chain in front of the word that `broke` says at which
/// name: `(by name, 1 match, chain broke at users)`, `by name, 2 declarations (chain broke at
/// users)`. A list that was `cut` short counts with a `+`, however few it holds.
pub(super) fn resolution(
    word: &str,
    target: Option<&str>,
    found: &[Candidate],
    broke: Option<&str>,
    cut: bool,
) -> String {
    let note = broke.map_or(String::new(), |at| format!(" (chain broke at {at})"));
    let Some(first) = found.first() else {
        return format!("no definition for {word}{note}");
    };
    let reason = &first.reason;
    if let ([_], false) = (found, cut) {
        let mut why = reason.to_string();
        if !reason.proven() {
            why.push_str(", 1 match");
        }
        if let Some(at) = broke {
            why.push_str(&format!(", chain broke at {at}"));
        }
        return match target {
            Some(target) => format!("{word} \u{2192} {target} ({why})"),
            None => format!("{word}: {why}"),
        };
    }
    // The candidates stop at MAX_HITS, so that many is a lower bound.
    let n = match found.len() {
        n if cut || n >= search::MAX_HITS => format!("{n}+"),
        n => n.to_string(),
    };
    if found.iter().all(|c| c.reason == *reason) {
        format!("{word}: {reason}, {n} declarations{note}")
    } else {
        // Each row says its own reason.
        format!("{word}: {n} declarations{note}")
    }
}
