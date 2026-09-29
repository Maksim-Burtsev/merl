//! `d` / F12: the word under the cursor, and where its declaration is looked for.

use super::*;

impl App {
    /// The word under the cursor as `d` and `u` ask about it: [`search::definition_word`], and
    /// a Ruby setter call `x.name = v` as the setter `name=` (#387); `def self.name = v` is a
    /// method of one line.
    pub(super) fn definition_word(
        &self,
        kind: Option<Kind>,
    ) -> Option<(std::ops::Range<usize>, String)> {
        let (range, word) = search::definition_word(kind, self.line_str(), self.col)?;
        let mut word = word.to_owned();
        let (written, start) = self.written(kind, range.start);
        let before = &written[..start];
        let after = self.line_str()[range.end..].trim_start();
        if kind == Some(Kind::Ruby)
            && before.ends_with('.')
            && !before.ends_with("..")
            && !before.trim_start().starts_with("def ")
            && !word.ends_with(['?', '!'])
            && after.starts_with('=')
            && !after[1..].starts_with(['=', '~', '>'])
        {
            word.push('=');
        }
        Some((range, word))
    }

    /// The cursor's line as `d` reads it, with where the word at byte `start` of it moved to: a
    /// member access broken over lines reads as the one line it is, in its plain access form.
    fn written(&self, kind: Option<Kind>, start: usize) -> (String, usize) {
        let (written, start) = kind
            .and_then(|k| search::unbroken(k, &self.buf.lines, self.line, start))
            .unwrap_or_else(|| (self.line_str().to_owned(), start));
        match kind {
            Some(k) => search::plain_access(k, &written, start),
            None => (written, start),
        }
    }

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
    /// enum variant outside Rust or a parameter has no declaration the rules know and gets "no
    /// definition": `u` lists the uses.
    pub(super) fn goto_definition(&mut self) {
        let kind = self.kind();
        // Markdown declares nothing: `d` follows the link under the cursor (#421).
        if kind == Some(Kind::Markdown)
            && let Some(here) = self.rel_current()
        {
            self.follow_markdown(&here);
            return;
        }
        // The path of a GraphQL `#import` is the file it pastes in, `./` and `/` included.
        if kind == Some(Kind::Graphql)
            && let Some(here) = self.rel_current()
            && let Some(module) = search::graphql_import(self.line_str(), self.col)
        {
            let module = module.to_owned();
            let files = search::module_files(
                Kind::Graphql,
                &self.root,
                &self.files,
                &here,
                std::slice::from_ref(&module),
            );
            let found = self.module_candidates(files);
            self.show_definitions(Kind::Graphql, &module, &here, found, None);
            return;
        }
        let Some((range, word)) = self.definition_word(kind) else {
            self.message = "no word".into();
            return;
        };
        let (written, start) = self.written(kind, range.start);
        let before = &written[..start];
        let dotted = before.ends_with('.') && !before.ends_with("..");
        let chain = search::qualifier(&written, start);
        let here = self.rel_current();
        let (Some(kind), Some(here)) = (kind, here) else {
            self.message = self.no_rules();
            return;
        };
        // Go's blank identifier names nothing: every `_` is a fresh discard (#476). Nor has a
        // GraphQL operation's `$variable` a rule: it is a parameter, and `$id` is no field `id`.
        // A Java or Kotlin class literal, `Foo::class`, names no member `class` (#362).
        if (kind == Kind::Go && word == "_")
            || (kind == Kind::Graphql && self.line_str()[..range.start].ends_with('$'))
            || (kind == Kind::Jvm
                && word == "class"
                && self.line_str()[..range.start].ends_with("::"))
        {
            self.message = resolution(&word, None, &[], None, false);
            return;
        }
        let text = self.buf.lines.join("\n");
        // Nothing in a Rust string names code, save the `{name}` a format string captures and
        // the path an attribute takes as a value, `#[serde(default = "default_port")]`: no
        // search for a word of prose (#346).
        let at = self.buf.lines[..self.line]
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + range.start;
        let line = self.line_str();
        let captured = line[..range.start].ends_with('{')
            && !line[..range.start].ends_with("{{")
            && line[range.end..].starts_with(['}', ':']);
        if !captured
            && search::in_string(kind, &text, at)
            && !search::rust_attribute_path(&self.buf.lines, self.line, range.start)
        {
            self.message = resolution(&word, None, &[], None, false);
            return;
        }
        // An import's path, and a name its package qualifies (#418).
        if kind == Kind::Proto
            && let Some(found) = self.proto_definitions(&text, &written[..start], &word)
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
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
        // `super` is no local, whatever the member lookup reads it as. A name of a destructuring
        // or a parameter list wrapped over several lines is on a line of its own (#393).
        let locals: Vec<usize> = search::bindings(kind, &text, self.line + 1, first)
            .iter()
            .map(|b| b.line)
            .filter(|&n| {
                (dotted || word != "super") && !names_itself(&self.buf.lines[n - 1], first)
            })
            .map(|n| match kind {
                Kind::TsJs => search::written_line(&self.buf.lines, n, first),
                _ => n,
            })
            .collect();
        if !locals.is_empty() {
            imports.retain(|(name, _)| name != first);
        }
        // The word itself is that parameter or local: its declarations in this scope are the
        // answer, and a function of the same name elsewhere is not. A Java or Kotlin name bound
        // earlier on the cursor's own line, `fun f(x: Int) = x`, is bound there (#376); behind a
        // `::` the word is a member, whatever the qualifier is.
        let same_line = kind == Kind::Jvm
            && locals == [self.line + 1]
            && whole_at(self.line_str(), &word, "").is_some_and(|at| at < range.start);
        if !dotted
            && !before.ends_with("::")
            && !locals.is_empty()
            && (locals != [self.line + 1] || same_line)
        {
            let found = locals
                .iter()
                .map(|&line| Candidate {
                    hit: Hit {
                        path: here.clone(),
                        line,
                        col: 0,
                        text: self.buf.lines[line - 1].clone(),
                    },
                    reason: Reason::Local,
                })
                .collect();
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A bare Java or Kotlin name no local binds: a member of the classes around the cursor,
        // innermost first, then of the class the innermost one extends (#376). Behind `new` it
        // is a constructor, which a nested class's line is not.
        let constructed = before
            .trim_end()
            .strip_suffix("new")
            .is_some_and(|b| !b.ends_with(is_word));
        if kind == Kind::Jvm
            && !constructed
            && chain.is_empty()
            && !dotted
            && !before.ends_with("::")
            && locals.is_empty()
            && let Some(found) = self.jvm_members(&here, &text, &word)
        {
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
                col: 0,
                text: self.buf.lines[line - 1].clone(),
            };
            let reason = Reason::Import(path.join("/"));
            self.show_definitions(kind, &word, &here, vec![Candidate { hit, reason }], None);
            return;
        }
        let own = matches!(chain.as_slice(), [s] if s == "self" || s == "cls" || s == "this");
        // Java and Kotlin's `Type::m` names a member of `Type` as `Type.m` does (#362). `super::m`
        // stays the search by name, and a bare `::m` has no chain.
        let referenced = kind == Kind::Jvm
            && before.ends_with("::")
            && chain.first().is_some_and(|f| f != "super");
        let on_value = (dotted || referenced)
            && !own
            && chain.first().is_none_or(|f| bound(&imports, f).is_none());
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
        // Rust's attributes, fields and variants, which the lines below cannot tell (#370).
        if kind == Kind::Rust
            && let Some(found) = self.rust_early(&here, &text, &word, range.clone(), dotted)
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // `x.word(…)` in Rust on a value whose type is not known (#358).
        if kind == Kind::Rust
            && on_value
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
        {
            let found = self.rust_methods(&here, &word);
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
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
        // A package no `node_modules` holds, no workspace package is called and no `declare
        // module` types is not installed (#392): nothing says what it declares, and the project's namesakes are not it. The
        // import line is where the name comes from, as far as anything tells. A copy of it among
        // the files outside, wherever they have it, is what the lookup outside reads, as before.
        if kind == Kind::TsJs
            && let Some((_, module)) = import.as_ref().and_then(|p| p.split_last())
            && search::package_missing(
                &self.root,
                &self.files,
                here.parent().unwrap_or(Path::new("")),
                module,
            )
            && !self.external_files(kind).iter().any(|f| {
                let parts = if module[0].starts_with('@') { 2 } else { 1 };
                search::in_module(f, &module[..parts.min(module.len())])
            })
            && let Some(line) = search::ts_import_line(&text, first)
        {
            let hit = Hit {
                path: here.clone(),
                line,
                col: 0,
                text: self.buf.lines[line - 1].clone(),
            };
            let reason = Reason::Import(format!("{} (not installed)", module.join("/")));
            self.show_definitions(kind, &word, &here, vec![Candidate { hit, reason }], None);
            return;
        }
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
        // A bare Rust variant behind a glob `use` of its enum (#370).
        if kind == Kind::Rust && !dotted && chain.is_empty() && locals.is_empty() {
            let found = self.rust_glob_variant(&here, &text, &word, &imports);
            if !found.is_empty() {
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
        }
        // A bare Rust name is an item of this file where the cursor sees it (#363).
        if kind == Kind::Rust && !dotted && chain.is_empty() && !before.ends_with("::") {
            let found = self.rust_file_items(&here, &text, &word, range.clone());
            if !found.is_empty() {
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
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
        // `this.m` and `this::m` in Java and Kotlin: `m` of the class around the cursor, which
        // this file declares (#362). Nothing of it declares `m`: the search by name, as before.
        if kind == Kind::Jvm
            && own
            && (dotted || before.ends_with("::"))
            && let Some(owner) = search::jvm_this_owner(&text, self.line + 1)
        {
            let full = format!("{owner}.{word}");
            let re = Regex::new(&pattern).expect("an escaped name keeps the pattern valid");
            let found: Vec<Candidate> = text
                .lines()
                .enumerate()
                .filter(|&(i, l)| {
                    re.is_match(l)
                        && search::qualified(kind, &text, i + 1, &word).as_ref() == Some(&full)
                })
                .map(|(i, l)| Candidate {
                    hit: Hit {
                        path: here.clone(),
                        line: i + 1,
                        col: 0,
                        text: l.to_owned(),
                    },
                    reason: Reason::Path("this".to_owned()),
                })
                .collect();
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
        // Ruby writes a constant's path with `::` and a method's with `.` (#387). The name a
        // `class A::B` line declares is no use of the path.
        let declared = !self.line_str()[range.end..].starts_with("::")
            && Regex::new(r"^\s*(?:class|module)\s+[\w:]*$").is_ok_and(|re| re.is_match(before));
        let colons = kind == Kind::Ruby && before.ends_with("::") && !declared;
        let pathed = match sep {
            "." => on_value || colons,
            _ => self.line_str()[..range.start].ends_with(&format!("{path}{sep}")),
        };
        // A type's name is no proof of which type (#129): in a `::` kind the project has to
        // declare it once, and a `use` of the file must not bind the path's first name to
        // somewhere outside, whatever `io.rs` the project has beside `std::io`. A Rust type the
        // project declares more than once is proven by the file of the module a `use crate::…`
        // takes it from (#227): an `impl` at the top of that file is for the type the path
        // names, declared there or handed on, and nothing else is offered then.
        let mut home: Option<Vec<PathBuf>> = None;
        // `Type::w` where this file declares the `Type` its scope sees, and another file one
        // too: this file's `Type` is the one, and its `Type::w` is looked for here first (#363).
        let seen = match kind == Kind::Rust && pathed && locals.is_empty() && chain.len() == 1 {
            true => search::rust_scope_items(
                &text,
                self.line + 1,
                &chain[0],
                search::RustNamespace::Path,
            ),
            false => Vec::new(),
        };
        // The `Type` the cursor sees, by the name its `impl` qualifies: `inner::Type` in a `mod`.
        let owners: Vec<String> = seen
            .iter()
            .map(|&l| search::qualified(kind, &text, l, &chain[0]).unwrap_or(chain[0].clone()))
            .collect();
        let mine = match owners.as_slice() {
            [owner, rest @ ..] if rest.iter().all(|o| o == owner) => {
                // A cut in this grep says nothing about the list shown in the end.
                let cut = self.truncated.get();
                let pattern = search::def_patterns(kind, &chain[0]).join("|");
                let elsewhere = self
                    .project_definitions(kind, &here, &chain[0], &pattern)
                    .iter()
                    .any(|h| h.path != here);
                self.truncated.set(cut);
                elsewhere.then(|| owner.clone())
            }
            _ => None,
        };
        if mine.is_some() {
            home = Some(vec![here.clone()]);
        }
        let pathed = pathed
            && !chain.is_empty()
            && (sep == "." || mine.is_some() || {
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
                let mut declared = self.project_definitions(kind, &here, owner, &owners);
                // A class is declared once, whatever its forward declarations and constructors
                // (#368).
                if kind == Kind::C {
                    declared =
                        search::c_type_rows(owner, declared, |p| self.text_of(p), false, false);
                }
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
        // Ruby's `Const.meth` is a class method (#387): a `def` of the class's instances is no
        // answer, and nor is a method of some other class found by name. `Const.new` runs the
        // class's `initialize`.
        // A constant in capitals holds a value, not a class: `REDIS_CONFIGURATION.cache`. An
        // acronym module (`JSON.parse`) reads so too, and stays the search by name.
        let class_method = kind == Kind::Ruby
            && dotted
            && chain.last().is_some_and(|c| {
                c.starts_with(|c: char| c.is_ascii_uppercase())
                    && c.contains(|c: char| c.is_ascii_lowercase())
            });
        let (word, pattern) = match class_method && word == "new" {
            true => (
                "initialize".to_owned(),
                search::def_patterns(kind, "initialize").join("|"),
            ),
            false => (word, pattern),
        };
        if pathed && locals.is_empty() && !chain.is_empty() {
            let full = format!("{path}{sep}{word}");
            // `depot::Shed::open` has the modules of the project in front of `Shed::open`.
            let in_project = sep != "." && self.names_module(&chain[0]);
            // Only the files of the module a `use` names can hold the answer then.
            let mut hits = match &home {
                Some(files) => self
                    .grep(&pattern, false, false, |p| files.iter().any(|f| f == p))
                    .unwrap_or_default(),
                None => self.project_definitions(kind, &here, &word, &pattern),
            };
            // `Mode::Auto`: an enum variant, which has no line pattern of its own (#370).
            if kind == Kind::Rust {
                let variants = self.rust_variants(&here, &word);
                hits.extend(
                    variants
                        .into_iter()
                        .filter(|v| home.as_ref().is_none_or(|files| files.contains(&v.path))),
                );
            }
            let singleton = class_method && word != "initialize";
            let concerns = singleton.then(|| hits.clone());
            let named: Vec<Candidate> = hits
                .into_iter()
                .filter(|h| {
                    let Some(t) = self.text_of(&h.path) else {
                        return false;
                    };
                    search::qualified(kind, &t, h.line, &word).is_some_and(|q| match &home {
                        Some(files) if mine.is_some() => {
                            mine.as_ref()
                                .is_some_and(|o| q == format!("{o}{sep}{word}"))
                                && files.contains(&h.path)
                        }
                        Some(files) => q == full && files.contains(&h.path),
                        None => {
                            q == full
                                || q.ends_with(&format!("{sep}{full}"))
                                || (in_project && full.ends_with(&format!("{sep}{q}")))
                        }
                    }) && (!singleton || search::ruby_singleton(&t, h.line))
                })
                .map(|hit| Candidate {
                    reason: match home {
                        Some(_) if mine.is_some() => Reason::Path(path.clone()),
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
            if class_method {
                let found: Vec<Candidate> = concerns
                    .map(|hits| self.concern_class_methods(kind, &here, &chain, hits))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Path(path.clone()),
                    })
                    .collect();
                // Nothing of the class declares it: a class method from outside the project
                // (ActiveRecord's `find`), or `Class#new` itself.
                let asked = if found.is_empty() && word == "initialize" {
                    "new"
                } else {
                    &word
                };
                self.show_definitions(kind, asked, &here, found, None);
                return;
            }
            // A cut in a grep whose result is dropped says nothing about the list below.
            self.truncated.set(false);
            // `Api::Fasp` that no declaration spells is no `::Fasp` elsewhere: what the name
            // alone finds is offered, not jumped to.
            self.offer_only |= colons;
        }
        // An Elixir qualifier a dependency declares as a module names that package, as an import
        // does elsewhere: its `def` there comes before a namesake of the project's (#437).
        if kind == Kind::Elixir && dotted && !chain.is_empty() && locals.is_empty() {
            let found = self
                .external_definitions(kind, &word, &chain, dotted, &imports, true)
                .unwrap_or_default();
            if !found.is_empty() {
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
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
        let hits = match kind {
            Kind::Jvm => self.jvm_seen(&here, hits, dotted || before.ends_with("::")),
            _ => hits,
        };
        // A C or C++ type is its body, not its forward declarations and constructors (#368).
        // Behind `.` or `->` no type is meant. Then what the file on screen can see (#364): not
        // another source file's statics and macros, not a `#define` that only stands in where
        // nothing else declares the name, and one definition over its prototypes.
        let mut fallbacks = Vec::new();
        let mut aside = None;
        let hits = match kind == Kind::C && !dotted && !before.ends_with("->") {
            true => {
                let (before, after) = (before.trim_end(), &self.line_str()[range.end..]);
                // `before` ends in the keyword `k` itself, not in a name ending so.
                let keyword = |k: &str| {
                    before.strip_suffix(k).is_some_and(|b| {
                        !b.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
                    })
                };
                let construction = after.trim_start().starts_with(['(', '{']) || keyword("new");
                let tag = ["struct", "union", "enum"].into_iter().any(keyword);
                // What a raw string or a block comment holds declares nothing, and must not count
                // as a second body or definition in the rules below (show_definitions drops it
                // again, for every kind).
                let mut literal: HashMap<PathBuf, Vec<bool>> = HashMap::new();
                let hits: Vec<Hit> = hits
                    .into_iter()
                    .filter(|h| {
                        let lines = literal.entry(h.path.clone()).or_insert_with(|| {
                            self.text_of(&h.path)
                                .map_or_else(Vec::new, |t| search::literal_lines(kind, &t))
                        });
                        !lines.get(h.line - 1).copied().unwrap_or(false)
                    })
                    .collect();
                let hits = search::c_type_rows(&word, hits, |p| self.text_of(p), construction, tag);
                let on = hits
                    .iter()
                    .any(|h| h.path == here && h.line == self.line + 1);
                let hits = search::c_file_local(&word, &here, &text, hits, |p| self.text_of(p), on);
                let (fallback, hits): (Vec<Hit>, Vec<Hit>) = hits.into_iter().partition(|h| {
                    self.text_of(&h.path)
                        .is_some_and(|t| search::c_fallback(&t, h.line, &word))
                });
                // With nothing else in the project, the search outside runs first.
                if hits.is_empty() {
                    fallbacks = fallback;
                }
                // On a candidate the others are offered, the prototype included.
                let parameter = search::c_parameter(&text, self.line + 1, &word);
                match search::c_one_definition(&word, &hits, |p| self.text_of(p))
                    .filter(|_| !on && !parameter)
                {
                    Some((at, note)) => {
                        aside = Some(note);
                        vec![hits[at].clone()]
                    }
                    None => hits,
                }
            }
            false => hits,
        };
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
        if found.is_empty() {
            found = fallbacks
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect();
        }
        // Ruby's core and gems are not read (#390): the one namesake the project declares of
        // `x.each` or `logger.info` proves nothing, and is offered rather than jumped to.
        self.offer_only |= kind == Kind::Ruby && on_value && self.external_files(kind).is_empty();
        self.show_definitions(kind, &word, &here, found, broke.as_deref());
        // The status of the one definition says what was set aside: `1 definition, 1 prototype`.
        if let Some(note) = aside {
            self.message = self.message.replacen("1 match", &note, 1);
        }
    }

    /// Of the Java or Kotlin declarations `hits` found by name, what the cursor can see (#357): a
    /// local only in its own block, below it, and never `behind` a `.` or a `::`; a `private`
    /// declaration only in its own file. When that drops some and leaves one that is no local
    /// in sight, it is still found by name only, and offered rather than jumped to.
    fn jvm_seen(&mut self, here: &Path, hits: Vec<Hit>, behind: bool) -> Vec<Hit> {
        let all = hits.len();
        let mut texts: HashMap<PathBuf, Option<String>> = HashMap::new();
        let mut seen_local = false;
        let kept: Vec<Hit> = hits
            .into_iter()
            .filter(|h| {
                if h.path != here && search::jvm_private(&h.text) {
                    return false;
                }
                // A declaration in column 0 is top-level, never a local.
                if !h.text.starts_with([' ', '\t']) {
                    return true;
                }
                let text = texts
                    .entry(h.path.clone())
                    .or_insert_with(|| self.text_of(&h.path));
                match text
                    .as_deref()
                    .and_then(|t| search::jvm_local_block(t, h.line))
                {
                    None => true,
                    Some(end) => {
                        let seen =
                            !behind && h.path == here && (h.line..=end).contains(&(self.line + 1));
                        seen_local |= seen;
                        seen
                    }
                }
            })
            .collect();
        self.offer_only |= kept.len() == 1 && kept.len() < all && !seen_local;
        kept
    }

    /// What a bare `word` names among the members of the Java or Kotlin classes around the
    /// cursor (#376): what the innermost class that declares it declares, `via` its name (an
    /// anonymous class's members are `local`), else what the class the innermost one extends
    /// declares, when the project declares that class once. `None` when neither does, or when the
    /// cursor stands on the member itself, whose namesakes and implementations are asked for.
    fn jvm_members(&self, here: &Path, text: &str, word: &str) -> Option<Vec<Candidate>> {
        let lines: Vec<&str> = text.lines().collect();
        let candidate = |path: &Path, lines: &[&str], line: usize, reason: Reason| Candidate {
            hit: Hit {
                path: path.to_path_buf(),
                line,
                col: 0,
                text: lines[line - 1].to_owned(),
            },
            reason,
        };
        let types = search::jvm_enclosing_types(text, self.line + 1);
        let mut found: Vec<Candidate> = Vec::new();
        for &decl in &types {
            let reason = match search::jvm_type_name(lines[decl - 1]) {
                Some(name) => Reason::Path(name),
                None => Reason::Local,
            };
            found = search::jvm_members_of(text, decl, word)
                .into_iter()
                .map(|line| candidate(here, &lines, line, reason.clone()))
                .collect();
            if !found.is_empty() {
                break;
            }
        }
        // One level up: the class the innermost named one extends, declared once.
        let inner = types
            .iter()
            .find(|&&d| search::jvm_type_name(lines[d - 1]).is_some());
        if found.is_empty()
            && let Some(&decl) = inner
        {
            for base in search::jvm_bases(text, decl) {
                let pattern = search::def_patterns(Kind::Jvm, &base).join("|");
                let cut = self.truncated.get();
                let declared: Vec<Hit> = self
                    .project_definitions(Kind::Jvm, here, &base, &pattern)
                    .into_iter()
                    .filter(|h| search::jvm_type_name(&h.text).as_deref() == Some(base.as_str()))
                    .collect();
                self.truncated.set(cut);
                let [hit] = declared.as_slice() else {
                    continue;
                };
                let Some(t) = self.text_of(&hit.path) else {
                    continue;
                };
                let base_lines: Vec<&str> = t.lines().collect();
                found.extend(
                    search::jvm_members_of(&t, hit.line, word)
                        .into_iter()
                        .map(|line| {
                            candidate(&hit.path, &base_lines, line, Reason::Path(base.clone()))
                        }),
                );
            }
        }
        let on_it = found
            .iter()
            .any(|c| c.hit.path == here && c.hit.line == self.line + 1);
        (!found.is_empty() && !on_it).then_some(found)
    }

    /// The first line of each of `files`, a module a name or a path leads to as a whole.
    fn module_candidates(&self, files: Vec<PathBuf>) -> Vec<Candidate> {
        files
            .into_iter()
            .map(|path| Candidate {
                reason: Reason::Module(path.display().to_string()),
                hit: Hit {
                    text: self.text_of(&path).map_or_else(String::new, |t| {
                        t.lines().next().unwrap_or_default().to_owned()
                    }),
                    path,
                    line: 1,
                    col: 0,
                },
            })
            .collect()
    }

    /// Of `hits`, the declarations of a Ruby method that a concern included by the class
    /// `chain` spells gives it as a class method (#387): `include Searchable` in a file declaring
    /// the class, and the method in `Searchable`'s `class_methods do` or `module ClassMethods`.
    fn concern_class_methods(
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
        // A Ruby superclass, right of the `<` of a `class` line, is a use of the name (#387): the
        // line declares another class.
        let superclass = kind == Kind::Ruby
            && search::definition_word(Some(kind), self.line_str(), self.col).is_some_and(
                |(r, _)| {
                    Regex::new(r"^\s*class\s+[\w:]+\s*<\s*[\w:]*$")
                        .is_ok_and(|re| re.is_match(&self.line_str()[..r.start]))
                },
            );
        if superclass {
            found.retain(|c| c.hit.line != self.line + 1 || c.hit.path != here);
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
                // A module's first line declares nothing of the word.
                let target = self
                    .text_of(&one.hit.path)
                    .filter(|_| !matches!(one.reason, Reason::Module(_)))
                    .and_then(|text| search::qualified(kind, &text, one.hit.line, word));
                let status = resolution(word, target.as_deref(), &found, broke, false);
                // The cursor lands on the word rather than at the start of the line, so a
                // second `d` there asks the next question about the same name: what
                // implements the declaration it just landed on (#68, step 6). A row of the
                // picker below lands there too, the word read by the rules `d` read it with.
                let extra = search::word_chars(Some(kind), true);
                self.jump_to_col(&path, one.hit.line, word_col(&one.hit.text, word, extra));
                // A refused jump (edits that cannot be saved) leaves its own reason, not a
                // resolution nobody followed.
                if self.buf.path.as_deref() == Some(path.as_path()) {
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
        // One file holds thousands of GraphQL `id` fields, so each file is split once.
        let mut lines: HashMap<PathBuf, Vec<String>> = HashMap::new();
        hits.retain(|h| {
            search::declares_where(kind, word, h.line, &h.text, || {
                lines.entry(h.path.clone()).or_insert_with(|| {
                    self.text_of(&h.path)
                        .map_or_else(Vec::new, |t| t.lines().map(str::to_owned).collect())
                })
            })
        });
        // `GO=$(GO) ./build.sh` in a recipe sets a variable of one shell command (#477): it
        // declares the word only for a shell variable of the command under the cursor,
        // `$${ARCH}`, and never for make's own `$(GO)`.
        if kind == Kind::Make {
            let line = self.line_str();
            let shell =
                search::definition_word(Some(kind), line, self.col).is_some_and(|(r, w)| {
                    let before = &line[..r.start];
                    let shell_ref = before.ends_with("$$(") || before.ends_with("$${");
                    let make_ref = before.ends_with("$(") || before.ends_with("${");
                    w == word && (shell_ref || !make_ref)
                });
            let text = self.buf.lines.join("\n");
            let command = search::make_recipe_command(&text, self.line + 1).filter(|_| shell);
            let recipe = |h: &Hit| {
                std::fs::read_to_string(self.root.join(&h.path))
                    .ok()
                    .and_then(|text| search::make_recipe_command(&text, h.line))
            };
            hits.retain(|h| match recipe(h) {
                None => true,
                at => h.path == here && at == command,
            });
            // `X += …` and `release: X := 1.0` set the variable only when nothing else does
            // (#499), so a jump to a plain assignment never becomes a picker. A recipe's `X+=1`
            // declares nothing, not even for the shell: `/bin/sh` has no `+=`.
            if hits.is_empty() {
                let fallback = search::make_fallback_patterns(word).join("|");
                hits = self
                    .grep(&fallback, false, false, |p| {
                        search::in_def_scope(kind, here, p)
                    })
                    .unwrap_or_default();
                self.note_cut(&hits);
                hits.retain(|h| recipe(h).is_none());
            }
        }
        hits
    }

    /// The declarations of `word` in the project module an import names, where `path` is what
    /// [`search::imports`] binds the word or its qualifier to; `None` when the module is not the
    /// project's. The word sits directly in the module, or in the class the chain goes through
    /// (`UserRepo.create`), as [`search::qualified`] names it: `store.Open` is never a method
    /// `Open`. An alias finds the imported name. A default import finds a declaration of its local
    /// name, else the module's `export default`. An empty list is a module that does not declare
    /// the word, a re-export: the search by name takes over. A Python name that is itself a
    /// module of the project, and that nothing on the way declares, is that module's file.
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
        // The word itself names a module of the project (#280): `views` in `from shop import
        // views`, `shop` in `import shop`. What Python imports: a package over a module of the
        // same name beside it. Only when nothing the lookup below reads declares or hands on
        // the name, so what a package's `__init__.py` binds keeps its say.
        let whole_module = || {
            let mut files = module_files(&tail(path.to_vec()));
            if files.iter().any(|f| f.ends_with("__init__.py")) {
                files.retain(|f| f.ends_with("__init__.py"));
            }
            let found = self.module_candidates(files);
            (!found.is_empty()).then_some(found)
        };
        let (files, inside) = match kind {
            // `from a import b` takes a module or a name from `a`: the longest module that exists,
            // never shorter than the one the import names.
            Kind::Python => {
                let target = tail(path.to_vec());
                let floor = path.len().saturating_sub(1).max(1);
                // A module importing from itself (`from . import views` in a package's
                // `__init__.py`) takes a module of the package, unless it declares the name
                // itself: then it is read as on master, where the import proves it.
                let declares = |name: &String| {
                    let pattern = search::def_patterns(kind, name).join("|");
                    self.grep(&pattern, false, false, |p| p == here)
                        .unwrap_or_default()
                        .iter()
                        .any(|h| {
                            self.text_of(&h.path).is_some_and(|text| {
                                search::qualified(kind, &text, h.line, name).is_none()
                            })
                        })
                };
                let found = (floor..target.len()).rev().find_map(|n| {
                    let files = module_files(&target[..n]);
                    let own = files == [here] && !target[n..].first().is_some_and(declares);
                    (!files.is_empty() && !own).then(|| (files, target[n..].to_vec()))
                });
                match found {
                    Some(found) => found,
                    None => return whole_module(),
                }
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
        // A module that binds the name in any way, an import that led nowhere included, is
        // left to the search by name.
        let unbound = |name: &String| {
            files.iter().all(|f| {
                self.text_of(f)
                    .is_none_or(|t| search::bindings(kind, &t, 1, name).is_empty())
            })
        };
        if hits.is_empty()
            && kind == Kind::Python
            && inside.first().is_some_and(unbound)
            && let Some(found) = whole_module()
        {
            return Some(found);
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
