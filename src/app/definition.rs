//! `d` / F12: the word under the cursor, and where its declaration is looked for.

use super::python::{assigns, python_functions};
use super::*;

impl App {
    /// The word under the cursor as `d` and `u` ask about it: [`search::definition_word`], and
    /// a Ruby setter call `x.name = v` as the setter `name=` (#387); `def self.name = v` is a
    /// method of one line.
    pub(super) fn definition_word(
        &self,
        kind: Option<Kind>,
    ) -> Option<(std::ops::Range<usize>, String)> {
        let (range, word) = self.word_here(kind)?;
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
        // Markdown declares nothing: `d` follows the link under the cursor (#421), on a deleted
        // line too, where it reads the line as the review draws it.
        if kind == Some(Kind::Markdown)
            && let Some(here) = self.rel_current()
        {
            self.follow_markdown(&here);
            return;
        }
        // Base code, a deleted line or a deleted file's, is looked up in the project as the base
        // had it (#440).
        if self.probe.is_none()
            && let Some((path, line)) = self.base_place()
        {
            self.definition_at_base(path, line);
            return;
        }
        // A class or an id of markup, and HTML and the stylesheets, read as #415 says.
        if let Some(here) = self.rel_current()
            && self.css_definition(kind, &here)
        {
            return;
        }
        if let Some(kind) = kind
            && let Some(here) = self.rel_current()
            && let Some(module) = search::file_import(kind, self.line_str(), self.col)
        {
            let module = [module];
            let files = self.import_files(kind, &here, &module);
            let found = self.module_candidates(files);
            self.show_definitions(kind, &module[0], &here, found, None);
            return;
        }
        let Some((range, mut word)) = self.definition_word(kind) else {
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
        if self.component_template(&here, &range, &mut word) {
            return;
        }
        // Go's blank identifier names nothing: every `_` is a fresh discard (#476). Nor has a
        // GraphQL operation's `$variable` a rule: it is a parameter, and `$id` is no field `id`.
        // A Java or Kotlin class literal, `Foo::class`, names no member `class` (#362). A
        // PowerShell `-Name` argument names a parameter of the command it is given to (#420).
        if (kind == Kind::Go && word == "_")
            || (kind == Kind::Graphql && self.line_str()[..range.start].ends_with('$'))
            || (kind == Kind::Jvm
                && word == "class"
                && self.line_str()[..range.start].ends_with("::"))
            || (kind == Kind::PowerShell
                && search::powershell_argument(&self.line_str()[..range.start]))
        {
            self.message = resolution(&word, None, &[], None, false);
            return;
        }
        let text = self.buf.lines.join("\n");
        let text = search::script_text(&here, &text, Some(self.line)).into_owned();
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
        // A segment of a Java or Kotlin `import` line: a package's declares nothing, a class's is
        // looked for in its package of the project (#372). A name inside a Scala import's `{…}`
        // selectors is looked for by name (#416).
        if kind == Kind::Jvm
            && import_line(kind, self.line_str())
            && !self.line_str()[..range.start].contains('{')
            && let Some(found) = self.jvm_imported(&text, &chain, &word, range.clone(), true)
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A segment of a C# `using` or `namespace` line names a namespace, and nothing else (#360).
        if kind == Kind::CSharp
            && let Some(found) = self.cs_namespace_segment(&here, &range)
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // An Elixir `alias` names the module a qualifier stands for (#459).
        let chain = match kind {
            Kind::Elixir => search::elixir_unalias(&text, self.line, chain),
            _ => chain,
        };
        if kind == Kind::Yaml
            && let Some(lines) =
                search::stage_entries(&self.line_str()[..range.start], &text, &word)
        {
            let found = lines
                .into_iter()
                .map(|line| Candidate {
                    hit: Hit {
                        deleted: None,
                        path: here.clone(),
                        line,
                        col: 0,
                        text: self.buf.lines[line - 1].clone(),
                    },
                    reason: Reason::ByName,
                })
                .collect();
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // An import's path, and a name its package qualifies (#418).
        if kind == Kind::Proto
            && let Some(found) = self.proto_definitions(&text, &written[..start], &word)
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        if let Some(found) = self.file_imported(kind, &here, &text, before, &chain, &word) {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A named argument, a literal's key or a JSX attribute names a parameter of the callee
        // or a field of the literal's type, and nothing else spelled so (#315, #316).
        if self.probe.is_none()
            && let java = here.extension().is_some_and(|e| e == "java")
            && let Some(label) = search::label_at(kind, java, &text, self.line, range.clone())
        {
            let found = label
                .owner
                .map(|(line, col, owner)| self.label_targets(kind, &here, &word, line, col, &owner))
                .unwrap_or_default();
            match found.is_empty() {
                true => self.message = format!("{word}: {}", label.what),
                false => self.show_definitions(kind, &word, &here, found, None),
            }
            return;
        }
        self.offer_only =
            kind == Kind::Python && search::keyword_argument(&text, self.line + 1, &range);
        self.truncated.set(false);
        // A PHP `$name` is a variable of its own function, never a function, a method or another
        // function's local (#464). `self::$x` is a static property, and `$this` the object.
        let sigil = &self.line_str()[..range.start];
        if kind == Kind::Php
            && sigil.ends_with('$')
            && !sigil.ends_with("::$")
            && word != "this"
            && let Some(lines) = search::php_variable(&text, self.line + 1, &word)
        {
            let found = lines
                .into_iter()
                .map(|line| Candidate {
                    hit: Hit {
                        deleted: None,
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
        // A C or C++ member behind `->` or `.`, or in a constructor's initializer list (#359).
        if kind == Kind::C
            && let Some((found, broke)) = self.c_early(&here, &text, &word, range.clone())
        {
            self.show_definitions(kind, &word, &here, found, broke.as_deref());
            return;
        }
        // A word in the module path of an import line names that module and nothing else
        // (#333): the project's, else the one outside, and never a namesake found by name.
        if kind == Kind::Python
            && let Some(module) = search::python_import_module(self.line_str(), range.start)
        {
            let found = self.python_module(&here, &module);
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A key of a Go composite literal is a field of the literal's type (#327). A literal whose
        // type is not read offers what the name finds, and never jumps to one.
        let go_key = match kind == Kind::Go && !dotted {
            true => search::go_key(&text, self.line + 1, range.start, range.end),
            false => search::GoKey::No,
        };
        let go_keyed = go_key != search::GoKey::No;
        // A receiver's or a literal's type may be declared outside the project, and is read
        // from the files walked there (#334).
        if kind == Kind::Go && (dotted || matches!(go_key, search::GoKey::Of(_))) {
            self.external_files(kind);
        }
        match go_key {
            search::GoKey::Of(written) => match self.literal_field(kind, &here, &written, &word) {
                Ok(Some(found)) => {
                    self.show_definitions(kind, &word, &here, found, None);
                    return;
                }
                Ok(None) => {}
                Err(()) => self.offer_only = true,
            },
            // A struct written in place: its body is on the lines above (#330).
            search::GoKey::Struct(line) => {
                let ty = typed::anonymous(&here, line);
                let found = self
                    .field_of(kind, &ty, &word, false)
                    .into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Receiver(ty.name.clone()),
                    })
                    .collect();
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
            search::GoKey::Unknown => self.offer_only = true,
            search::GoKey::No | search::GoKey::Value => {}
        }
        // A Ruby `@name` or `@@name` is a variable of the class around the cursor, assigned in
        // its methods (#383): a word of its own, never the bare `name`, and never another class's.
        let lead = &self.line_str()[..range.start];
        if kind == Kind::Ruby && lead.ends_with('@') {
            let sigil = match lead.ends_with("@@") {
                true => "@@",
                false => "@",
            };
            let ivar = format!("{sigil}{word}");
            let found = self.ruby_ivars(&here, &text, &ivar);
            self.show_definitions(kind, &ivar, &here, found, None);
            return;
        }
        // Inside a docstring's example the imports written there count too.
        let in_literal = search::literal_lines(kind, &text).get(self.line) == Some(&true);
        let mut imports = match in_literal {
            true => search::imports_as_written(kind, &text),
            false => search::imports(kind, &text),
        };
        // A parameter or a local of the same name hides the import where the cursor is: `json`
        // in `def handler(json)` is a value, and `via import` would be a proof of nothing.
        let first = chain.first().map_or(word.as_str(), String::as_str);
        // A key of a Lua table constructor names a field, whatever local shares its name, and so
        // does a method called with `:`.
        let key = kind == Kind::Lua
            && (search::table_key(&text, self.line + 1, &range)
                || (before.ends_with(':') && !before.ends_with("::")));
        // An Elixir name followed by `(` is a call, one behind `@` an attribute and behind `&` a
        // capture, and the names an `@spec` promises are functions: each is its module's first
        // (#460). Nor is a key `name:` or an atom `:name` a variable.
        let after = &self.line_str()[range.end..];
        let call = kind == Kind::Elixir
            && !dotted
            && (after.starts_with('(')
                || before.ends_with(['@', '&'])
                || before.trim_start().starts_with("@spec "));
        let key = key
            || call
            || (kind == Kind::Elixir
                && !dotted
                && (before.ends_with(':') || (after.starts_with(':') && !after.starts_with("::"))));
        // `super` is no local, whatever the member lookup reads it as; Lua has no `super`. A name
        // of a destructuring or a parameter list wrapped over several lines is on a line of its
        // own (#393).
        // A Rust macro, a path and its segments are no local, nor a field a literal or a pattern
        // names with `w:` (a format string's `{w:?}` is the local), save on the line that
        // declares the local (#353). On the pattern of a
        // `let` the word is that local itself.
        let after = self.line_str()[range.end..].trim_start();
        let rust_path = kind == Kind::Rust
            && (before.ends_with("::")
                || after.starts_with("::")
                || (after.starts_with('!') && !after.starts_with("!=")));
        let rust_field =
            kind == Kind::Rust && after.starts_with(':') && !after.starts_with("::") && !captured;
        // On the name a Swift `for`, `if let`, closure or function header declares on the
        // cursor's own line, the word is at a declaration, as on a `let` (#525, #533): the
        // occurrence the line no longer binds the name without. A read in front of it, `cache`
        // in `if cache.isEmpty, let cache = load() {`, is looked up as on any other line, and an
        // outer binding of the name is a namesake, never the answer.
        let line = self.line_str();
        let swift_binds_here = kind == Kind::Swift
            && !dotted
            && line.get(range.clone()) == Some(word.as_str())
            && search::swift_binds_on(line, &word)
            && !search::swift_binds_on(
                &format!("{}_{}", &line[..range.start], &line[range.end..]),
                &word,
            );
        let declared = swift_binds_here
            || (kind == Kind::Rust
                && chain.is_empty()
                && search::rust_let_declares(self.line_str(), range.start, &word));
        // A TypeScript bare word is a value a `class`, `function`, `type`, `interface` or `enum`
        // of its scope declares as well as a `const` (#337); the first name of a chain is not.
        let bare = kind == Kind::TsJs && !dotted && chain.is_empty();
        let required = match kind {
            Kind::TsJs => search::ts_import_lines(&text, first),
            _ => Vec::new(),
        };
        let closure = |n: usize| {
            (kind == Kind::Swift || kind == Kind::Python && search::python_in_function(&text, n))
                && !dotted
                && chain.is_empty()
                && n != self.line + 1
        };
        let locals_at = |text: &str, line: usize| -> Vec<usize> {
            let binding: Vec<usize> = match declared {
                true => vec![self.line + 1],
                false => search::bindings(kind, text, line, first)
                    .iter()
                    .map(|b| b.line)
                    .collect(),
            };
            binding
                .into_iter()
                .filter(|&n| {
                    let l = &self.buf.lines[n - 1];
                    !key && (dotted || word != "super" || kind == Kind::Lua)
                        && !(import_line(kind, l)
                            || (!bare && names_itself(kind, l, first) && !closure(n)))
                        && !rust_path
                        && (!rust_field || n == self.line + 1)
                })
                .map(|n| match kind {
                    Kind::TsJs => search::written_line(&self.buf.lines, n, first),
                    Kind::CSharp => search::cs_written_line(&self.buf.lines, n, first),
                    _ => n,
                })
                // `const { helper } = require("./m")` binds an import, as `import` does (#328).
                .filter(|n| !required.contains(n))
                .collect()
        };
        let mut locals = locals_at(&text, self.line + 1);
        // A Go parameter read in a body on its function's own line is that parameter, whatever
        // the scopes around it bind (#524).
        let go_own = kind == Kind::Go
            && locals.contains(&(self.line + 1))
            && search::go_binds_here(self.line_str(), &word, range.start);
        if go_own {
            locals = vec![self.line + 1];
        }
        if (bare || self.script_scope(&here, Some(first)).is_some()) && locals.is_empty() {
            let module = locals_at(&format!("{text}\n0"), self.buf.lines.len() + 1);
            if !module.contains(&(self.line + 1)) {
                locals = module;
            }
        }
        // An arrow function on the cursor's line whose body holds the cursor binds the word
        // there, whatever the scopes around it declare (#531).
        let own_arrow = kind == Kind::TsJs
            && !dotted
            && locals.contains(&(self.line + 1))
            && search::ts_arrow_binds(self.line_str(), &word, range.start);
        // On a parameter an arrow on the cursor's line declares, the word is at a declaration
        // (#534): that line is its binding, as a function's parameter list is, and the
        // namesakes are left to the search by name below.
        let arrow_param = kind == Kind::TsJs
            && !dotted
            && search::ts_arrow_param(self.line_str(), &word, range.start);
        if own_arrow || arrow_param {
            locals = vec![self.line + 1];
        }
        if !locals.is_empty() {
            imports.retain(|(name, _)| name != first);
        }
        // A Python builtin nothing in the file binds has no source to land on (#336): not the
        // project's namesake in another module, which the bare name does not reach without an
        // import, nor a method of a dependency. A `*` import may bind it, and so may the class
        // body the cursor is in: `render = format` beside its `def format`.
        let star = kind == Kind::Python && imports.iter().any(|(name, _)| name == "*");
        let unbound = kind == Kind::Python
            && !dotted
            && chain.is_empty()
            && search::bindings(kind, &text, self.line + 1, &word).is_empty()
            && !search::python_class_binds(&text, self.line + 1, &word)
            && !names_itself(kind, self.line_str(), &word)
            && !self.offer_only;
        if unbound && !star && search::PYTHON_BUILTINS.contains(&word.as_str()) {
            self.offer_only = false;
            self.message = format!("{word}: builtin, no source");
            return;
        }
        let on_itself = match kind {
            Kind::C => search::c_bindings_at(&text, self.line + 1, first)
                .iter()
                .all(|&(l, c)| l == self.line + 1 && c == range.start),
            _ => locals == [self.line + 1],
        };
        let same_line = (matches!(kind, Kind::Jvm | Kind::PowerShell)
            || (kind == Kind::Swift && !declared))
            && locals == [self.line + 1]
            && whole_at(self.line_str(), &word, "").is_some_and(|at| at < range.start);
        let own_line = go_own
            || (kind == Kind::CSharp
                && locals == [self.line + 1]
                && search::cs_binds_here(self.line_str(), &word, range.start));
        if !dotted
            && !before.ends_with("::")
            && !locals.is_empty()
            && (!on_itself
                || same_line
                || own_line
                || own_arrow
                || matches!(kind, Kind::Rust | Kind::Nix | Kind::Haskell))
        {
            let found = locals
                .iter()
                .map(|&line| Candidate {
                    hit: Hit {
                        deleted: None,
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
        // A bare name inside a C++ method is a field of its class first (#378).
        if kind == Kind::C
            && !dotted
            && chain.is_empty()
            && locals.is_empty()
            && !before.ends_with("::")
            && let Some(class) = search::c_method_class(&text, self.line + 1, &word)
        {
            let found = self.c_class_fields(&here, &word, &class, false);
            if !found.is_empty() {
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
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
            let found = self.jvm_fit_candidates(&here, &word, range.clone(), &chain, found);
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A Java or Kotlin name read through the file's imports and packages (#372).
        if kind == Kind::Jvm
            && locals.is_empty()
            && !(dotted && chain.is_empty())
            && !names_itself(kind, self.line_str(), &word)
            && let Some(found) = self.jvm_imported(&text, &chain, &word, range.clone(), false)
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A Lua qualifier a `require` or a table constructor gives (#462). Standing on the one
        // declaration it finds, the namesakes by name are offered, as before.
        if kind == Kind::Lua {
            let chain = match before.strip_suffix(':') {
                Some(b) if !b.ends_with(':') => search::qualifier(&format!("{b}."), start),
                _ => chain.clone(),
            };
            let found = self.lua_qualified(&here, &text, &chain, &word);
            let on = |c: &Candidate| c.hit.path == here && c.hit.line == self.line + 1;
            if !found.iter().all(on) {
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
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
                deleted: None,
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
        // A Java or Kotlin receiver whose declaration writes its type (#388, #391): the member of
        // that type, and nothing for a type outside the project. A string literal is a `String`.
        if kind == Kind::Jvm && (dotted || before.ends_with("::")) && !own {
            let literal = dotted && chain.is_empty() && before[..before.len() - 1].ends_with('"');
            let found = match literal {
                true => self.jvm_outside(&here, &word, "String").map(|hits| {
                    hits.into_iter()
                        .map(|hit| Candidate {
                            hit,
                            reason: Reason::Receiver("String".into()),
                        })
                        .collect()
                }),
                false => self.jvm_typed(&here, &text, &chain, &word),
            };
            if let Some(found) = found {
                let found = self.jvm_fit_candidates(&here, &word, range.clone(), &chain, found);
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
        }
        // Java and Kotlin's `Type::m` names a member of `Type` as `Type.m` does (#362). `super::m`
        // stays the search by name, and a bare `::m` has no chain.
        let referenced = kind == Kind::Jvm
            && before.ends_with("::")
            && chain.first().is_some_and(|f| f != "super");
        // PHP reaches a member with `->` alone, whatever the chain starts with: `Offer::Cut->value`
        // is a value's member too (#348). Behind `->` a call is a method, anything else a property.
        let php_members = (kind == Kind::Php && dotted).then(|| {
            let call = self.line_str()[range.end..].trim_start().starts_with('(');
            search::php_member_patterns(&word, call).join("|")
        });
        let on_value = (dotted || referenced)
            && !own
            && (kind == Kind::Php || chain.first().is_none_or(|f| bound(&imports, f).is_none()));
        let members = match &php_members {
            Some(m) => on_value.then(|| m.clone()),
            None => on_value
                .then(|| search::member_patterns(kind, &word))
                .flatten()
                .map(|m| m.join("|")),
        };
        let mut patterns = search::def_patterns(kind, &word);
        // Where only a C# type can stand, only a type's rules count (#360): a type, a delegate, an
        // alias, and no constructor, property or namespace of the name (save a constant, #581).
        let cs_type = kind == Kind::CSharp
            && !dotted
            && chain.is_empty()
            && self.cs_types_only(&here, &word, range.clone());
        if cs_type {
            patterns = search::cs_type_patterns(&word);
        }
        // A Ruby local is seen from its own method or block alone, and a value has none: its
        // assignments are this file's where the cursor sees them, below, never a search by name
        // (#383). A constant is the project's.
        let ruby_local =
            kind == Kind::Ruby && word.starts_with(|c: char| c.is_ascii_lowercase() || c == '_');
        if kind == Kind::Ruby && (dotted || ruby_local) {
            let assignment = search::ruby_assignment(&word);
            patterns.retain(|p| *p != assignment);
        }
        // A Ruby call with no receiver that no local names, and `self.meth`, is a method of
        // `self`: its class, the modules it mixes in, its superclasses, in that order, before any
        // namesake by name (#365).
        if ruby_local
            && locals.is_empty()
            && ((!dotted && chain.is_empty() && !before.ends_with("::")) || own)
            && search::ruby_locals(&text, self.line + 1, &word).is_empty()
        {
            let found = self.ruby_self_methods(&here, &text, &word);
            if !found.is_empty() {
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
            // A cut in a grep whose result is dropped says nothing about the list below.
            self.truncated.set(false);
        }
        // An Elixir call is never a module attribute: `Shop.currency()` is no `@currency` (#459).
        if kind == Kind::Elixir && dotted {
            patterns.retain(|p| !p.starts_with(r"^\s*@"));
        }
        search::narrow_patterns(kind, &mut patterns, &text, self.line_str(), range.clone());
        if patterns.is_empty() {
            self.message = self.no_rules();
            return;
        }
        let pattern = patterns.join("|");
        // PHP's `$this->`, `self::`, `static::`, `parent::`, typed receivers and class names.
        if kind == Kind::Php
            && let Some(found) = self.php_early(&here, &text, before, &chain, &word, range.clone())
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // `x.word` in Rust on a receiver whose type is proven (#377): the word of that type.
        let mut rust_broke = None;
        if kind == Kind::Rust
            && dotted
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
            && chain.first().is_some_and(|f| bound(&imports, f).is_none())
            // A tuple field, `self.0`, has no rule.
            && chain.iter().all(|n| n.starts_with(|c: char| c.is_alphabetic() || c == '_'))
        {
            let after = &self.line_str()[range.end..];
            let call = after.starts_with('(') || after.starts_with("::<");
            match self.rust_typed(&here, &text, &word, &chain, call) {
                Ok(found) if !found.is_empty() => {
                    self.show_definitions(kind, &word, &here, found, None);
                    return;
                }
                Ok(_) => {}
                // With one name in front of the word, `by name` already says where.
                Err(at) => rust_broke = (chain.len() > 1).then_some(at),
            }
        }
        // Rust's attributes, fields and variants, which the lines below cannot tell (#370).
        if kind == Kind::Rust
            && let Some(found) = self.rust_early(&here, &text, &word, range.clone(), dotted)
        {
            self.show_definitions(kind, &word, &here, found, rust_broke.as_deref());
            return;
        }
        // `x.word(…)` in Rust on a value whose type is not known (#358).
        if kind == Kind::Rust
            && on_value
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
        {
            let found = self.rust_methods(&here, &word);
            self.show_definitions(kind, &word, &here, found, rust_broke.as_deref());
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
        // A bare Go name is a local, a name of the file's package, of a dot import or a
        // predeclared one, and nothing else (#332). A literal's key, a name an import binds and
        // the declaration under the cursor keep their lookup.
        let mut declaring = search::def_patterns(kind, &word);
        declaring.extend(search::member_or_signature(kind, &word).unwrap_or_default());
        let declares_here =
            Regex::new(&declaring.join("|")).is_ok_and(|re| re.is_match(self.line_str()));
        if kind == Kind::Go
            && !dotted
            && !go_keyed
            && locals.is_empty()
            && !declares_here
            && bound(&imports, &word).is_none()
        {
            let found = self.go_bare(&here, &word, &imports);
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A key of an object literal the file binds to a `const`, `X.key`, is declared inside
        // that literal (#341).
        if kind == Kind::TsJs
            && dotted
            && let [name] = chain.as_slice()
            && let [binding] = locals.as_slice()
            && let Some(line) = search::ts_literal_key(&self.buf.lines, *binding, name, &word)
        {
            let hit = Hit {
                deleted: None,
                path: here.clone(),
                line,
                col: 0,
                text: self.buf.lines[line - 1].clone(),
            };
            let found = vec![Candidate {
                hit,
                reason: Reason::Local,
            }];
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A bare Swift word in a type's body, once the walk proves it no local, is a member of
        // that type through the implicit `self`, or of the class it extends, first (#380).
        if kind == Kind::Swift
            && !dotted
            && chain.is_empty()
            && (locals.is_empty() && search::swift_no_local(&text, self.line + 1, &word))
            && !declares_here
            && !matches!(word.as_str(), "self" | "super" | "init")
            && let Some(found) = self.swift_self_members(&here, &text, &word, &pattern)
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A Swift receiver whose type is proven: that type's member, or on a type from outside
        // that only the project's extensions reach, those or nothing (#384).
        if kind == Kind::Swift
            && dotted
            && !chain.is_empty()
            && chain[0] != "super"
            && word != "init"
            && bound(&imports, &chain[0]).is_none()
            && let Some(found) = self.swift_typed(&here, &text, &word, &chain)
        {
            match found.is_empty() {
                true => self.message = resolution(&word, None, &[], None, false),
                false => self.show_definitions(kind, &word, &here, found, None),
            }
            return;
        }
        // A receiver whose type is proven narrows the member to that type (#68, steps 2 to 4).
        let mut broke = None;
        let mut cs_walked = None;
        // C# writes its types everywhere: a member of an object initializer's type, or of the
        // type a receiver is declared with; a type the project does not declare is the
        // framework's, and so is its member (#352).
        if kind == Kind::CSharp {
            let answer = match dotted {
                false => self.cs_initializer(&here, &word, &range).map(Ok),
                true if !chain.is_empty() && bound(&imports, &chain[0]).is_none() => {
                    Some(self.cs_typed(&here, &word, &chain))
                }
                true => None,
            };
            match answer {
                Some(Ok(cs_typed::CsAnswer::Found(found))) => {
                    self.show_definitions(kind, &word, &here, found, None);
                    return;
                }
                Some(Ok(cs_typed::CsAnswer::Outside(via))) => {
                    self.offer_only = false;
                    self.truncated.set(false);
                    self.message = format!("no definition for {word} in the project (via {via})");
                    return;
                }
                Some(Err(at)) => broke = (chain.len() > 1).then_some(at),
                None => {}
            }
            // A bare name is the type's around it first, as C# resolves it (#360).
            if !dotted && chain.is_empty() && locals.is_empty() && !before.ends_with("::") {
                match self.cs_class_first(&here, &word, cs_type) {
                    Ok(found) => {
                        self.show_definitions(kind, &word, &here, found, None);
                        return;
                    }
                    Err(walked) => cs_walked = walked.filter(|_| !declares_here),
                }
            }
        }
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
                // What the class lacks can come from its bases outside the project, or from a
                // project class extending it, which may set it on `self` (#342): only those
                // subclasses' declarations of the name are candidates.
                Ok(_) => {
                    if let Some(ty) = self.inherited_outside(kind, &here, &chain, head.as_ref()) {
                        let found = self.subclass_members(kind, &here, &word, &pattern, &ty);
                        self.show_definitions(kind, &word, &here, found, None);
                        return;
                    }
                }
                // With one name in front of the word, `by name` already says where.
                Err(at) => {
                    // A value proven to be a builtin type: its members have no source (#336).
                    if let Some(via) = self.builtin_receiver(kind, &here, &chain, head.as_ref()) {
                        self.offer_only = false;
                        self.truncated.set(false);
                        self.message = format!("{word}: builtin, no source (via {via})");
                        return;
                    }
                    let names = head.as_ref().map_or(chain.len(), |(_, _, f)| f.len() + 1);
                    broke = (names > 1).then_some(at);
                }
            }
        }
        // A Rust path's first name names the crate searched first, and in it the module the path
        // spells (#350): a `use` of `std::fs::File` takes `File::open` to the standard library,
        // whatever `open` the project declares.
        if kind == Kind::Rust
            && !dotted
            && locals.is_empty()
            && let Some(found) = self.rust_crate_path(&here, &text, &word, &chain, {
                let after = self.line_str()[range.end..].trim_start();
                match after.starts_with('!') && !after.starts_with("!=") {
                    true => Some(true),
                    false => after.starts_with(['(', ':']).then_some(false),
                }
            })
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
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
        // A barrel of the project that hands the name on from a package, `export { x } from
        // "lodash"`, leads into that package, as an import straight from it does (#527), when
        // nothing of the project answers the import.
        let barrel = import
            .as_deref()
            .and_then(|path| self.barrel_package(kind, &here, &word, &chain, path));
        let import = match barrel {
            Some(package) => {
                for (name, p) in &mut imports {
                    if name.as_str() == first {
                        *p = package.clone();
                    }
                }
                Some(package)
            }
            None => import,
        };
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
                deleted: None,
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
        // The import names a module of the project that was read.
        let mut project_read = false;
        // The module outside holding the value the import names, whose members are not read
        // (#560).
        let mut value: Option<Vec<String>> = None;
        let mut found = match import {
            Some(path) => {
                let project = self.imported_definitions(kind, &here, &word, &chain, &path);
                project_read = project.is_some();
                // The name itself, bare or as a qualifier, bound to a module outside (#333).
                if project.is_none()
                    && kind == Kind::Python
                    && !dotted
                    && chain.is_empty()
                    && imports.iter().filter(|(name, _)| *name == word).count() == 1
                    && let Some(found) = self.bound_module(&path)
                {
                    self.show_definitions(kind, &word, &here, found, None);
                    return;
                }
                // `User.objects` behind an import from outside, `User` a class of the module
                // (#342): a member of `User` there, never a top-level `objects` anywhere. A value
                // the module holds, Django's `settings` (#560): no namesake outside is its member,
                // the project's search by name answers, as Django reads the project's settings.
                if project.is_none() && kind == Kind::Python && dotted && chain.len() == 1 {
                    match self.outside_class_member(&word, &path) {
                        Some(Some(found)) => {
                            self.show_definitions(kind, &word, &here, found, None);
                            return;
                        }
                        Some(None) => value = Some(path[..path.len() - 1].to_vec()),
                        None => {}
                    }
                }
                let mut found = project.unwrap_or_else(|| {
                    if value.is_some() {
                        return Vec::new();
                    }
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
        // Go's `pkg.X` is declared in `pkg`'s directory or nowhere: Go has no re-exports (#332).
        // Cgo's `C` has no directory. `pkg.Var.Method` goes through a value, and a package the
        // lookup could not map while the project holds its directory (a `vendor/` copy) was not
        // read: both keep the search by name.
        let go_qualified = kind == Kind::Go
            && chain.len() == 1
            && bound(&imports, &chain[0]).is_some_and(|p| {
                p != ["C"]
                    && (project_read || !self.files.iter().any(|f| search::in_package(f, &p)))
            });
        if !found.is_empty() || go_qualified {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A member of a JavaScript or DOM global that no scope, import or declaration of the
        // project binds, `JSON.parse`, is [`Self::js_global_members`], never a class's (#341).
        if kind == Kind::TsJs
            && locals.is_empty()
            && let [global] = chain.as_slice()
            && super::imported::JS_GLOBALS.contains(&global.as_str())
            && bound(&imports, global).is_none()
            && self
                .project_definitions(
                    kind,
                    &here,
                    global,
                    &search::def_patterns(kind, global).join("|"),
                )
                .is_empty()
        {
            let found = self.js_global_members(&here, &word, members.as_deref(), &pattern);
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
                        deleted: None,
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
            // Every row names the one `Drawer::Scanner`, so its forward declaration in the class
            // yields to its body, `struct Drawer::Scanner {` (#508): read as outside any class.
            // On the body itself, the declaration is the other end to go to.
            let on_row = named
                .iter()
                .any(|c| c.hit.path == here && c.hit.line == self.line + 1);
            let named = match kind == Kind::C && !on_row {
                true => {
                    let rows: Vec<Hit> = named.iter().map(|c| c.hit.clone()).collect();
                    let kept = search::c_type_rows(&word, rows, |_| None, true, false);
                    named
                        .into_iter()
                        .filter(|c| {
                            kept.iter()
                                .any(|h| h.path == c.hit.path && h.line == c.hit.line)
                        })
                        .collect()
                }
                false => named,
            };
            // Swift's `Type.word` reaches a case or a `static` member: an instance member needs a
            // value (#380). A type with only instance members of the name keeps them.
            let named = match kind == Kind::Swift
                && named
                    .iter()
                    .any(|c| !search::swift_instance_member(&c.hit.text))
            {
                true => named
                    .into_iter()
                    .filter(|c| !search::swift_instance_member(&c.hit.text))
                    .collect(),
                false => named,
            };
            if !named.is_empty() {
                self.show_definitions(kind, &word, &here, named, None);
                return;
            }
            // Java's and Kotlin's `Offer.CUT`: a constant of the enum the project declares once
            // (#457).
            if let (Kind::Jvm, [owner]) = (kind, chain.as_slice()) {
                let found = self.enum_constant(kind, &here, owner, &word);
                if !found.is_empty() {
                    self.show_definitions(kind, &word, &here, found, None);
                    return;
                }
            }
            if class_method {
                let (asked, found) =
                    self.ruby_class_method_elsewhere(&here, &chain, &word, &pattern, concerns);
                self.show_definitions(kind, &asked, &here, found, None);
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
        // An Elixir call or attribute is its own module's first (#460). On a declaration of the
        // name, its namesakes are offered as before. A call is never an attribute nor a struct's
        // field, and an attribute is nothing else. Past an `import` that may bring the name in
        // (no `only:`, or one listing it) a call is looked up by name, as on master: which of
        // the two it means turns on an arity this rule does not count.
        let attribute = before.ends_with('@');
        let imported = !attribute
            && text.lines().any(|l| {
                l.trim_start().starts_with("import ")
                    && (!l.contains("only:") || l.contains(&format!("{word}:")))
            });
        let module_pattern = search::def_patterns(kind, &word)
            .into_iter()
            .filter(|p| match attribute {
                true => p.starts_with(r"^\s*@"),
                false => !p.starts_with(r"^\s*@") && !p.contains("defstruct"),
            })
            .collect::<Vec<_>>()
            .join("|");
        if call
            && !imported
            && let Ok(re) = Regex::new(&module_pattern)
        {
            let lines = search::elixir_module_lines(&text, self.line + 1, &re);
            if !lines.is_empty() && !lines.contains(&(self.line + 1)) {
                let found = lines
                    .into_iter()
                    .map(|line| Candidate {
                        hit: Hit {
                            deleted: None,
                            path: here.clone(),
                            line,
                            col: 0,
                            text: self.buf.lines[line - 1].clone(),
                        },
                        reason: Reason::File,
                    })
                    .collect();
                self.show_definitions(kind, &word, &here, found, None);
                return;
            }
        }
        // A C# `Offer.Cut` whose qualifier is an `enum` of the project is a member its body
        // lists (#466).
        if kind == Kind::CSharp
            && pathed
            && locals.is_empty()
            && let Some(found) = self.cs_enum_member(&here, &text, &chain, &word, &path)
        {
            self.show_definitions(kind, &word, &here, found, None);
            return;
        }
        // A C# `Task.Delay` whose first name the project declares nowhere, in no form, is a
        // member of a type outside it (#355): NuGet ships assemblies, so nothing names what the
        // project's namesakes are not. A lowercase first name is a value of an unknown type.
        if kind == Kind::CSharp
            && dotted
            && locals.is_empty()
            && let Some(first) = chain.first()
            && first.starts_with(|c: char| c.is_ascii_uppercase())
            && !self.cs_declares(&here, first)
        {
            self.show_definitions(kind, &word, &here, Vec::new(), None);
            return;
        }
        // A parameter or a local in front of the word is a value for certain: it has members,
        // and a function or a variable at the top of a module is not one of them.
        // Nor is anything but a member one of PHP's, where `$x->` is always a value (#348).
        let hits = members
            .as_ref()
            .map(|m| self.members_by_name(kind, &here, &word, m))
            .filter(|hits| !hits.is_empty() || !locals.is_empty() || kind == Kind::Php)
            .unwrap_or_else(|| {
                if own {
                    // `self.word` whose class is not read to the end: the declarations of the name,
                    // and the fields too (#104). PHP's `$this->word` is a member (#348).
                    let pattern = php_members.as_ref().unwrap_or(&pattern);
                    self.members_by_name(kind, &here, &word, pattern)
                } else {
                    self.project_definitions(kind, &here, &word, &pattern)
                }
            });
        // A bare Python name never calls a method (#522): a `def` in a class is reached through a
        // value or the class, or seen bare from that class's own body. On a declaration of the
        // name its namesakes stay.
        let mut hits = hits;
        if kind == Kind::Python
            && !dotted
            && chain.is_empty()
            && !declares_here
            && !search::python_class_binds(&text, self.line + 1, &word)
        {
            let all = hits.len();
            let mut kept = hits.clone();
            kept.retain(|h| {
                let d = h.text.trim_start();
                !(d.starts_with("def ") || d.starts_with("async def "))
                    || !self
                        .text_of(&h.path)
                        .is_some_and(|t| search::python_method(&t, h.line))
            });
            // The one row left when its rivals went is jumped to, so the name must reach it
            // from the cursor: at module level, or in a function around the cursor. Another
            // function's nested `def` is no more the answer than the methods were, and the
            // picker stays as it was.
            let around = python_functions(&text, self.line + 1);
            let unreachable = |h: &Hit| {
                let inside = self
                    .text_of(&h.path)
                    .and_then(|t| python_functions(&t, h.line).first().copied());
                inside.is_some_and(|f| h.path != here || !around.contains(&f))
            };
            if !matches!(kept.as_slice(), [h] if all > 1 && unreachable(h)) {
                hits = kept;
            }
        }
        // A Swift implicit member, `.word` with no name in front, is an enum case or a `static`
        // member of the type expected there, and in a `case` pattern a case alone (#380). When
        // the project declares none of them, the answer is outside it and the rows stay.
        if kind == Kind::Swift && dotted && chain.is_empty() && word != "init" {
            let dot = &written[..start - 1];
            let lead = dot.trim_end();
            let implicit = lead.is_empty()
                || lead.ends_with([
                    '(', ',', '[', ':', '=', '<', '>', '&', '|', '+', '-', '*', '/', '%', '^', '~',
                ])
                || (lead.len() < dot.len() && lead.ends_with(['?', '!']))
                || ["return", "case", "let", "var"]
                    .iter()
                    .any(|k| lead.strip_suffix(k).is_some_and(|b| !b.ends_with(is_word)));
            let pattern = written.trim_start().starts_with("case ");
            let kept: Vec<Hit> = hits
                .iter()
                .filter(|h| {
                    search::swift_case(&h.text)
                        || (!pattern && search::swift_static_member(&h.text) == Some(true))
                })
                .cloned()
                .collect();
            if implicit && !kept.is_empty() {
                hits = kept;
            }
        }
        if kind == Kind::Swift && !dotted && chain.is_empty() {
            self.swift_nested_unseen(&mut hits);
        }
        // A Lua `local` inside a block is seen by that block alone, where the bindings above
        // found it already: anywhere else, and behind a dot, it is no candidate (#461). What is
        // left was a namesake beside it on master, and is offered, never jumped to. The cursor's
        // own line stays, standing on a declaration.
        if ruby_local && !dotted && chain.is_empty() {
            hits.extend(
                search::ruby_locals(&text, self.line + 1, &word)
                    .into_iter()
                    .map(|line| Hit {
                        deleted: None,
                        path: here.clone(),
                        line,
                        col: 0,
                        text: self.buf.lines[line - 1].clone(),
                    }),
            );
        }
        // What a C# name reaches of its namesakes (#355, #360).
        if kind == Kind::CSharp {
            let chain = dotted.then_some(chain.as_slice());
            hits = self.cs_reachable(&here, &word, chain, cs_walked.as_deref(), hits);
        }
        if kind == Kind::Lua {
            let at = |h: &Hit| h.path == here && h.line == self.line + 1;
            let all = hits.len();
            hits.retain(|h| {
                !h.text.starts_with([' ', '\t'])
                    || !h.text.trim_start().starts_with("local ")
                    || at(h)
            });
            self.offer_only |= hits.len() < all && hits.iter().any(|h| !at(h));
        }
        // A Swift function's `let` or `var` is seen inside that function and the functions
        // nested in it alone (#371): behind a `.` it is no member, and for a bare word anywhere
        // else it is another function's local.
        // The cursor's own line alone is offered rather than jumped to when others went: the
        // word may be a use on the line of a declaration of its name (#317).
        // The parameter itself is among its namesakes, so they are offered, never jumped to
        // (#534).
        if arrow_param
            && !hits
                .iter()
                .any(|h| h.path == here && h.line == self.line + 1)
        {
            hits.push(Hit {
                path: here.clone(),
                line: self.line + 1,
                col: 0,
                text: self.line_str().to_owned(),
                deleted: None,
            });
        }
        if kind == Kind::Swift {
            // On the name a binding declares on the cursor's own line (`swift_binds_here`), the
            // line joins the namesakes found by name, as a `let` line does by its pattern.
            let own = Hit {
                path: here.clone(),
                line: self.line + 1,
                col: 0,
                text: self.line_str().to_owned(),
                deleted: None,
            };
            if swift_binds_here
                && !hits
                    .iter()
                    .any(|h| h.path == own.path && h.line == own.line)
            {
                hits.push(own);
            }
            let all = hits.len();
            let lines: Vec<&str> = text.lines().collect();
            let literal = search::literal_lines(kind, &text);
            // The cursor's scope and the functions around it, innermost first; a header's scope
            // lies above it, so the walk ends.
            let mut scopes = vec![search::swift_scope(&lines, &literal, self.line + 1).0];
            while let Some(&s @ 1..) = scopes.last() {
                match search::swift_scope(&lines, &literal, s).0 {
                    0 => break,
                    up => scopes.push(up),
                }
            }
            let mut files: HashMap<PathBuf, (Vec<String>, Vec<bool>)> = HashMap::new();
            hits.retain(|h| {
                let (lines, literal) = files.entry(h.path.clone()).or_insert_with(|| {
                    let t = self.text_of(&h.path).unwrap_or_default();
                    let literal = search::literal_lines(kind, &t);
                    (t.lines().map(str::to_owned).collect(), literal)
                });
                match search::swift_local(lines, literal, h.line) {
                    Some(at) => !dotted && h.path == here && scopes.contains(&at),
                    None => true,
                }
            });
            self.offer_only |= hits.len() < all
                && matches!(hits.as_slice(), [h] if h.path == here && h.line == self.line + 1);
        }
        let hits = match kind {
            Kind::Jvm => {
                let hits = self.jvm_seen(&here, hits, dotted || before.ends_with("::"));
                let hits = self.jvm_use_fit(&here, &word, range.clone(), &chain, hits);
                // `values()` found its enum: no namesake left out of sight makes it a guess.
                if matches!(word.as_str(), "values" | "valueOf")
                    && matches!(hits.as_slice(), [h] if h.text.contains("enum "))
                {
                    self.offer_only = false;
                }
                hits
            }
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
                // A value is followed by `->` or `.`, and is never a type (#378).
                let after = after.trim_start();
                let value = locals.is_empty()
                    && (after.starts_with("->")
                        || (after.starts_with('.') && !after.starts_with("..")));
                let hits: Vec<Hit> = hits
                    .into_iter()
                    .filter(|h| !value || !Self::c_type_line(&word, h))
                    .collect();
                let tag = ["struct", "union", "enum"].into_iter().any(keyword);
                // What a raw string, a block comment or a macro's body holds declares nothing,
                // and must not count as a second body or definition in the rules below.
                let mut literal = HashMap::new();
                let hits: Vec<Hit> = (hits.into_iter())
                    .filter(|h| self.c_code_line(&mut literal, h))
                    .collect();
                let hits = search::c_type_rows(&word, hits, |p| self.text_of(p), construction, tag);
                let hits = self.c_reached_only(&here, &word, hits);
                let on = hits
                    .iter()
                    .any(|h| h.path == here && h.line == self.line + 1);
                let hits = search::c_file_local(&word, &here, &text, hits, |p| self.text_of(p), on);
                // On an out-of-line `R X::name(…) {`, the other end is what X's body declares of
                // the name, overloads included, and no namesake by name (#373).
                let line = self.line_str();
                let declared: Vec<Option<String>> = hits
                    .iter()
                    .map(|h| {
                        let scopes = self
                            .text_of(&h.path)
                            .filter(|_| on && before.ends_with("::"))
                            .and_then(|t| search::c_member_class(&t, h.line, &word))?;
                        search::c_defines_member(line, &scopes, &word)
                            .then(|| scopes.last().cloned())
                            .flatten()
                    })
                    .collect();
                if declared.iter().any(Option::is_some) {
                    let found = hits
                        .into_iter()
                        .zip(declared)
                        .filter_map(|(hit, owner)| {
                            Some(Candidate {
                                hit,
                                reason: Reason::Path(owner?),
                            })
                        })
                        .collect();
                    self.show_definitions(kind, &word, &here, found, None);
                    return;
                }
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
        // A member of a value outside is no method or field of a project class, nor a function:
        // only a module-level `NAME = …` of the project, which is what Django's `settings` reads
        // (#560). ponytail: one under an `if` at module level goes unseen, a miss. What the
        // project does not set, the defaults do: a module-level `NAME = …` in the package of the
        // module, as `django/conf/global_settings.py` beside `django/conf/__init__.py`.
        if let Some(module) = &value {
            found.retain(|c| assigns(&c.hit.text, &word));
            if found.is_empty() {
                found = self.package_assignments(&word, module);
            }
        }
        // A Swift `extension X` declares no `X` (#371). With the type in the project the
        // extensions are no candidates; with only extensions, `X` is declared outside: in a
        // dependency, else in Foundation or the standard library, which ship no source, and the
        // extensions are offered then, never jumped to.
        if kind == Kind::Swift && !found.is_empty() {
            let extends = |c: &Candidate| search::swift_extension(&c.hit.text);
            if found.iter().any(|c| !extends(c)) {
                found.retain(|c| !extends(c));
            } else {
                let mut away = self
                    .external_definitions(kind, &word, &chain, dotted, &imports, false)
                    .unwrap_or_default();
                away.retain(|c| !extends(c));
                match away.is_empty() {
                    true => self.offer_only = true,
                    false => found = away,
                }
            }
        }
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
            // One method outside is no proof while a field of the name is declared outside
            // too, which is never listed (#342): the one method is offered, counted `1+`.
            let (external, field) = match kind {
                Kind::Python => self.external_methods(&files, members, &word),
                _ => (self.external_grep(kind, &files, members), false),
            };
            if found.is_empty() && external.len() == 1 && field {
                self.truncated.set(true);
            }
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
            && value.is_none()
            && !matches!(
                chain.first().map(String::as_str),
                Some("self" | "cls" | "this")
            )
        {
            // After the project, outside as the import names the module, every installed copy
            // of it: what a workspace package linked in hands on from a dependency is there.
            // A bare Python name nothing binds is no method, and outside the project only a
            // module the file `*`-imports can bind it (#336).
            found = match unbound && !search::PYTHON_BUILTINS.contains(&word.as_str()) {
                true => self.star_imported(&word, &imports),
                false => self
                    .external_definitions(kind, &word, &chain, dotted, &imports, false)
                    .unwrap_or_default(),
            };
            // `group->pel` opens no system `struct group` (#378).
            let after = self.line_str()[range.end..].trim_start();
            if kind == Kind::C
                && locals.is_empty()
                && (after.starts_with("->") || (after.starts_with('.') && !after.starts_with("..")))
            {
                found.retain(|c| !Self::c_type_line(&word, &c.hit));
            }
            // The module the import loads is the project's, searched already: nothing outside is
            // proven to be what it hands on.
            if own_module {
                for c in &mut found {
                    c.reason = Reason::ByName;
                }
            }
        }
        // A Zig local is no candidate outside its function, nor a declaration without `pub`
        // outside its file (#469); the one under the cursor stays, its own answer.
        if kind == Kind::Zig {
            found.retain(|c| {
                (c.hit.path == here && c.hit.line == self.line + 1)
                    || self
                        .text_of(&c.hit.path)
                        .is_some_and(|t| search::zig_visible(&t, c.hit.line, c.hit.path == here))
            });
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
        // Nothing in the branch by name: the definitions the branch deleted are the answer (#440).
        // A rule that answered earlier, "none" included, keeps its answer.
        // No method of the name: the field Lombok writes the accessor for (#381).
        if kind == Kind::Jvm && found.is_empty() {
            found = self.jvm_lombok(&here, &text, &chain, &word);
        }
        if found.is_empty() && self.probe.is_none() {
            found = self.deleted_definitions(kind, &word, &here);
        }
        // A Ruby value's member: what Ruby's core and the gems declare too, offered (#369, #390).
        if kind == Kind::Ruby && on_value {
            self.ruby_member_outside(&word, &pattern, &mut found);
        }
        self.show_definitions(kind, &word, &here, found, broke.as_deref());
        // The status of the one definition says what was set aside: `1 definition, 1 prototype`.
        if let Some(note) = aside {
            self.message = self.message.replacen("1 match", &note, 1);
        }
    }

    /// Where the label `word` lands (#316): `d` resolves the name at 0-based `line` and byte
    /// `col` as it would there, and each declaration of it in the project gives the parameter
    /// or the field the label names. Several are overloads; none leaves the label unresolved.
    fn label_targets(
        &mut self,
        kind: Kind,
        here: &Path,
        word: &str,
        line: usize,
        col: usize,
        owner: &search::Owner,
    ) -> Vec<Candidate> {
        let (line0, col0) = (self.line, self.col);
        let message = std::mem::take(&mut self.message);
        (self.line, self.col) = (line, col);
        let name = self.definition_word(Some(kind)).map(|(_, w)| w);
        let text = self.buf.lines.join("\n");
        let owners = match name
            .as_deref()
            .and_then(|n| search::own_type(kind, &text, line, n))
        {
            // `new self(…)`: the class the cursor is in.
            Some(decl) => vec![Candidate {
                hit: Hit {
                    deleted: None,
                    path: here.to_path_buf(),
                    line: decl,
                    col: 0,
                    text: self.buf.lines[decl - 1].clone(),
                },
                reason: Reason::File,
            }],
            None => {
                self.probe = Some(Vec::new());
                self.goto_definition();
                self.probe.take().unwrap_or_default()
            }
        };
        (self.line, self.col, self.message) = (line0, col0, message);
        // A callee found only by name, or one of whose candidates lies outside the project, where
        // nothing is read, may be another namesake's: its parameter is offered, never jumped to.
        // A callee whose receiver's type the project does not declare, or one of whose candidates
        // lies outside the project, where nothing is read, may be a library's namesake: its
        // parameter is offered, never jumped to. A typed chain that broke says so for Python.
        // A Rust struct outside is read for its fields: a literal of it names only `pub` ones
        // (#529).
        let reads_outside = kind == Kind::Rust && *owner == search::Owner::Typed;
        let offer = std::mem::take(&mut self.offer_only)
            || (!reads_outside && owners.iter().any(|c| c.hit.path.is_absolute()))
            || (owners.iter().all(|c| !c.reason.proven())
                && name
                    .as_deref()
                    .is_some_and(|n| self.receiver_outside(kind, here, line, col, n)));
        self.truncated.set(false);
        let Some(name) = name else {
            return Vec::new();
        };
        let what = match owner {
            search::Owner::Args => "parameter",
            search::Owner::Object(_) | search::Owner::Typed => "field",
        };
        let mut out: Vec<Candidate> = Vec::new();
        for c in owners {
            // Outside the project nothing is read; the line under the cursor declares nothing
            // it calls.
            if (c.hit.path.is_absolute() && !reads_outside)
                || (c.hit.path == here && c.hit.line == line0 + 1)
            {
                continue;
            }
            let Some(text) = self.text_of(&c.hit.path) else {
                continue;
            };
            let of = search::qualified(kind, &text, c.hit.line, &name).unwrap_or(name.clone());
            let mut at: Vec<(PathBuf, usize, String)> =
                search::label_lines(kind, &text, c.hit.line, &name, word, owner)
                    .into_iter()
                    .map(|l| (c.hit.path.clone(), l, of.clone()))
                    .collect();
            // `<Tag a={…}>` of `function Tag(p: Props)`: the key is `Props`'s, when the project
            // declares one `Props`.
            if at.is_empty()
                && let search::Owner::Object(n) = owner
                && let Some(ty) = search::object_type(kind, &text, c.hit.line, &name, *n)
            {
                let cut = self.truncated.get();
                let pattern = search::def_patterns(kind, &ty).join("|");
                let decls = self.project_definitions(kind, here, &ty, &pattern);
                self.truncated.set(cut);
                if let [d] = decls.as_slice()
                    && let Some(t) = self.text_of(&d.path)
                {
                    at = search::label_lines(kind, &t, d.line, &ty, word, &search::Owner::Typed)
                        .into_iter()
                        .map(|l| (d.path.clone(), l, ty.clone()))
                        .collect();
                }
            }
            let by_name = match c.reason.proven() {
                true => "",
                false => ", found by name",
            };
            for (path, line, of) in at {
                if out.iter().any(|o| o.hit.path == path && o.hit.line == line) {
                    continue;
                }
                let text = self
                    .text_of(&path)
                    .and_then(|t| t.lines().nth(line - 1).map(str::to_owned))
                    .unwrap_or_default();
                out.push(Candidate {
                    hit: Hit {
                        deleted: None,
                        path,
                        line,
                        col: 0,
                        text,
                    },
                    reason: Reason::Label(format!("{what} of {of}{by_name}")),
                });
            }
        }
        self.offer_only = offer && !out.is_empty();
        out
    }

    /// Whether the receiver of the call whose callee `name` ends at byte `col` of 0-based `line`
    /// has a type written for it that the project does not declare: `client` of `client.get(`,
    /// declared `client: HttpClient` or `HttpClient client` on a line above.
    fn receiver_outside(
        &self,
        kind: Kind,
        here: &Path,
        line: usize,
        col: usize,
        name: &str,
    ) -> bool {
        let lines = &self.buf.lines;
        let Some(head) = lines[line]
            .get(..col + 1)
            .and_then(|h| h.strip_suffix(name))
            .and_then(|h| h.strip_suffix('.'))
        else {
            return false;
        };
        let recv = head
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next()
            .unwrap_or_default();
        let r = regex::escape(recv);
        let Ok(re) = Regex::new(&format!(
            r"\b{r}\s*:\s*([A-Z][\w.]*)|\b([A-Z][\w.]*)(?:<[^>]*>)?\s+{r}\b"
        )) else {
            return false;
        };
        let ty = lines[..=line].iter().rev().find_map(|l| {
            let c = re.captures(l)?;
            let t = c.get(1).or(c.get(2))?.as_str();
            t.rsplit('.').next().map(str::to_owned)
        });
        let Some(ty) = ty.filter(|_| !recv.is_empty()) else {
            return false;
        };
        let cut = self.truncated.get();
        let pattern = search::def_patterns(kind, &ty).join("|");
        let none = self
            .project_definitions(kind, here, &ty, &pattern)
            .is_empty();
        self.truncated.set(cut);
        none
    }

    /// The declarations of `word` by name (`pattern`, and the fields), in `ty` and the project
    /// classes that extend it, four levels down (#342). `ty` itself counts for what the typed
    /// walk does not read: a nested class, a `def` under an `if`.
    fn subclass_members(
        &mut self,
        kind: Kind,
        here: &Path,
        word: &str,
        pattern: &str,
        ty: &Typed,
    ) -> Vec<Candidate> {
        let mut owners = self.subtypes(kind, here, ty);
        owners.push((ty.path.clone(), ty.line));
        self.members_by_name(kind, here, word, pattern)
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    let lines: Vec<&str> = t.lines().collect();
                    search::enclosing_type(kind, &lines, h.line - 1)
                        .is_some_and(|d| owners.contains(&(h.path.clone(), d)))
                })
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
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
                deleted: None,
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
            for base in search::jvm_bases(text, decl, search::scala(here)) {
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

    /// The field `word` of the Go struct a literal `written{…}` builds, `file` the file writing it
    /// (#327): only the struct's own fields, since Go takes no promoted field as a key. An empty
    /// list is a struct without the field, `None` a type that is no struct (a named map or slice,
    /// whose keys are values), `Err` a type the rules do not find or read.
    fn literal_field(
        &self,
        kind: Kind,
        file: &Path,
        written: &str,
        word: &str,
    ) -> Result<Option<Vec<Candidate>>, ()> {
        let Some(ty) = self.struct_decl(kind, file, written, 0)? else {
            return Ok(None);
        };
        let label = format!(
            "{}{{\u{2026}}}",
            written.split('[').next().unwrap_or(written)
        );
        Ok(Some(
            self.field_of(kind, &ty, word, false)
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect(),
        ))
    }

    /// The Go struct the type written as `written` in `file` is: itself, or what a defined
    /// `type X Y` is over, eight deep. `None` for a type that is no struct, `Err` for one the
    /// rules do not find or read.
    pub(super) fn struct_decl(
        &self,
        kind: Kind,
        file: &Path,
        written: &str,
        depth: usize,
    ) -> Result<Option<Typed>, ()> {
        static DEFINED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^type\s+[A-Za-z_]\w*(?:\[[^\]]*\])?\s+([^/{]*)").unwrap()
        });
        let ty = self.type_decl(kind, file, written).ok_or(())?;
        let text = self.text_of(&ty.path).ok_or(())?;
        let line = text.lines().nth(ty.line - 1).ok_or(())?;
        let over = DEFINED.captures(line).ok_or(())?[1].trim().to_owned();
        if over.starts_with("struct") {
            return Ok(Some(ty));
        }
        let named = search::type_path(kind, &over).is_some() && !over.starts_with('*');
        match named && depth < 8 {
            true => self.struct_decl(kind, &ty.path, &over, depth + 1),
            false => Ok(None),
        }
    }

    /// Where a bare Go name no scope of the file declares is declared (#332): at the top level
    /// of the file's package, else in the package of a dot import, else in GOROOT's
    /// `builtin/builtin.go`, the predeclared `len` and `error`.
    fn go_bare(
        &mut self,
        here: &Path,
        word: &str,
        imports: &[(String, Vec<String>)],
    ) -> Vec<Candidate> {
        let kind = Kind::Go;
        let by = |hits: Vec<Hit>, reason: Reason| -> Vec<Candidate> {
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect()
        };
        let own = self.package_declarations(kind, here, word, None);
        let own = self.host_built(kind, own);
        if !own.is_empty() {
            return by(own, Reason::ByName);
        }
        let dot = [".".to_owned()];
        for (_, path) in imports.iter().filter(|(name, _)| name == ".") {
            let found = match self.imported_definitions(kind, here, word, &dot, path) {
                Some(found) => found,
                None => {
                    let only = [(".".to_owned(), path.clone())];
                    self.external_definitions(kind, word, &dot, false, &only, true)
                        .unwrap_or_default()
                }
            };
            if !found.is_empty() {
                return found;
            }
        }
        self.external_files(kind);
        let builtin: Vec<PathBuf> = self
            .external
            .get(&kind)
            .and_then(|(roots, _)| roots.first())
            .map(|goroot| goroot.join("builtin/builtin.go"))
            .filter(|f| f.is_file())
            .into_iter()
            .collect();
        let pattern = search::def_patterns(kind, word).join("|");
        let hits = self.external_grep(kind, &builtin, &pattern);
        by(
            self.declaring(kind, word, hits),
            Reason::Path("builtin".to_owned()),
        )
    }

    /// Lua's `m.word`, `m.T.word`, `T.word` (#462): `m` bound by `local m = require("a.b")` reads
    /// `a/b.lua` or `a/b/init.lua`, at the root or under `lua/`, and the table it returns; `T` is
    /// a table that file, or the one on screen, declares. The declarations of `word` in that
    /// table are the answer. A qualifier these rules cannot read gives nothing, and the search
    /// by name goes on.
    fn lua_qualified(
        &self,
        here: &Path,
        text: &str,
        chain: &[String],
        word: &str,
    ) -> Vec<Candidate> {
        let Some((first, rest)) = chain.split_first() else {
            return Vec::new();
        };
        let Some(value) = search::lua_local_value(text, self.line + 1, first) else {
            return Vec::new();
        };
        let required = search::lua_required(&value);
        let (path, reason) = match required {
            Some(module) => {
                let dir = module.replace('.', "/");
                let Some(path) = ["", "lua/"]
                    .iter()
                    .flat_map(|root| [format!("{root}{dir}.lua"), format!("{root}{dir}/init.lua")])
                    .map(PathBuf::from)
                    .find(|p| self.files.contains(p))
                else {
                    return Vec::new();
                };
                (path, Reason::Import(module.to_owned()))
            }
            None if value.starts_with('{') => (here.to_path_buf(), Reason::Path(chain.join("."))),
            None => return Vec::new(),
        };
        let Some(source) = self.text_of(&path) else {
            return Vec::new();
        };
        let mut table = match required {
            Some(_) => search::lua_returned(&source),
            None => Some(first.clone()),
        };
        for name in rest {
            let Some(next) = search::lua_table(&source, table.as_deref(), name) else {
                return Vec::new();
            };
            table = Some(next);
        }
        let Some(table) = table else {
            return Vec::new();
        };
        search::lua_members(&source, &table, word)
            .into_iter()
            .map(|line| Candidate {
                hit: Hit {
                    deleted: None,
                    path: path.clone(),
                    line,
                    col: 0,
                    text: source.lines().nth(line - 1).unwrap_or_default().to_owned(),
                },
                reason: reason.clone(),
            })
            .collect()
    }

    /// `word` as a constant of the Java or Kotlin enum `owner`, when the project declares one type
    /// of that name and it is an `enum` (#457), and this file sees it: it is in the enum's package,
    /// or imports the enum or its whole package (`.*`). `import java.util.concurrent.TimeUnit`, or
    /// `java.util.concurrent.*` alone, is not the project's `TimeUnit`.
    fn enum_constant(&self, kind: Kind, here: &Path, owner: &str, word: &str) -> Vec<Candidate> {
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
        // A type imported by name from elsewhere hides the package's own.
        let elsewhere = imports.iter().any(|(p, all)| !all && *p != home);
        let sees =
            package(&self.buf.lines.join("\n")) == home || imports.iter().any(|(p, _)| *p == home);
        if elsewhere || !sees {
            return Vec::new();
        }
        let lines: Vec<&str> = text.lines().collect();
        search::enum_constants(&text, decl.line)
            .into_iter()
            .filter(|(name, _)| name == word)
            .map(|(_, line)| Candidate {
                hit: Hit {
                    deleted: None,
                    path: decl.path.clone(),
                    line,
                    col: 0,
                    text: lines.get(line - 1).copied().unwrap_or_default().to_owned(),
                },
                reason: Reason::Path(owner.to_owned()),
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
    pub(super) fn show_definitions(
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
        // `it("works", async () => {` and `check(` over `line,` over `);` are calls shaped like
        // a method's header (#343), however many: a test suite has thousands of `it(`.
        if kind == Kind::TsJs {
            let head = search::ts_method_head(word);
            found.retain(|c| {
                !search::ts_call_statement(&head, &c.hit.text, c.hit.line, || {
                    self.text_of(&c.hit.path)
                })
            });
        }
        if found.len() <= 500 {
            let mut literal: HashMap<(PathBuf, bool), Vec<bool>> = HashMap::new();
            found.retain(|c| {
                let lines = (literal.entry((c.hit.path.clone(), c.hit.deleted.is_some())))
                    .or_insert_with(|| self.hidden_now(kind, &c.hit));
                // `register<` over its type arguments over `>(1);` is a call prettier wrapped.
                // A type's header wrapped so declares the type: `class User extends Model<`,
                // `export interface Context<` (#331).
                let call = kind == Kind::TsJs
                    && c.hit.text.trim_end().ends_with('<')
                    && !search::declares_type(kind, &c.hit.text)
                    && !Regex::new(r"\b(?:extends|implements)\b")
                        .is_ok_and(|re| re.is_match(&c.hit.text))
                    && self
                        .text_of(&c.hit.path)
                        .is_some_and(|t| !search::declares_wrapped_generic(&t, c.hit.line));
                // A tag of a PHP class's docblock (#344) or of a JavaScript `@typedef` (#347)
                // is a declaration inside a comment.
                // A name a component's template binds is declared there (#413).
                let literal = lines.get(c.hit.line - 1).copied().unwrap_or(false)
                    && !self.doc_tag(kind, &c.hit)
                    && !(c.reason == Reason::Local && search::component(&c.hit.path));
                !call && !literal
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
        if kind == Kind::C {
            self.c_rows(word, here, &mut found);
        }
        // A Go parameter's type named like the method it is a parameter of, `Send(msg Send)`, is
        // looked up as a type (#536); with no type found, the namesakes stay offered.
        let as_type = kind == Kind::Go
            && search::definition_word(Some(kind), self.line_str(), self.col).is_some_and(
                |(r, w)| w == word && search::go_param_type(self.line_str(), word, r.start),
            )
            && found
                .iter()
                .any(|c| search::declares_type(kind, &c.hit.text));
        if as_type {
            found.retain(|c| search::declares_type(kind, &c.hit.text));
        }
        if kind == Kind::Python {
            self.python_implementation(word, here, &mut found);
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
        let namesakes = !as_type
            && found.iter().all(|c| !c.reason.proven())
            && !found.is_empty()
            && (found.len() < all || on_member())
            // Alone, the declaration under the cursor is its own answer.
            && found
                .iter()
                .any(|c| c.hit.line != self.line + 1 || c.hit.path != here)
            && self.on_declared_name(kind, word, false);
        // Off the name of the line's declaration (#317), a bare word whose lone namesake nothing
        // proves and is declared as this line declares it, `let courier` of another function, is
        // as likely another scope's copy as what the word means: offered, as before, never
        // jumped to. Behind a `.` it is a member, which no local is.
        let extra = search::word_chars(Some(kind), true);
        let form = |t: &str| t[..word_col(t, word, extra)].trim().to_owned();
        let copy = found.len() < all
            && !namesakes
            && search::definition_word(Some(kind), self.line_str(), self.col)
                .is_some_and(|(r, _)| !self.line_str()[..r.start].ends_with('.'))
            && matches!(found.as_slice(), [c] if !c.reason.proven() && form(&c.hit.text) == form(self.line_str()));
        // Tests, mocks, fixtures, generated and vendored copies of a declaration come last here
        // too (#81) — except in the file on screen, which is what the reader is reading. The sort
        // is stable and every candidate is a declaration, so the rest keep the order the search
        // found them in, standard library and all.
        found.sort_by_cached_key(|c| search::rank(&c.hit.path, Some(here), true).0);
        // The project and the outside are each cut at MAX_HITS; the picker holds that many.
        found.truncate(search::MAX_HITS);
        if let Some(probe) = &mut self.probe {
            *probe = found;
            self.offer_only = offer_only || broke.is_some();
            return;
        }
        match found.as_slice() {
            [] => self.message = resolution(word, None, &found, broke, truncated),
            [one] if !namesakes && !offer_only && !truncated && !copy => {
                let path = self.root.join(&one.hit.path);
                // A module's first line declares nothing of the word, and a label's reason names
                // what declares the parameter or the field already.
                let name = declared_as(kind, word, &one.hit.text);
                let target = self
                    .hit_text(&one.hit)
                    .filter(|_| !matches!(one.reason, Reason::Module(_) | Reason::Label(_)))
                    .and_then(|text| search::qualified(kind, &text, one.hit.line, &name));
                let status = resolution(word, target.as_deref(), &found, broke, false);
                // The cursor lands on the word rather than at the start of the line, so a
                // second `d` there asks the next question about the same name: what
                // implements the declaration it just landed on (#68, step 6). A row of the
                // picker below lands there too, the word read by the rules `d` read it with.
                let extra = search::word_chars(Some(kind), true);
                let col = word_col(&one.hit.text, &name, extra);
                match one.hit.deleted {
                    Some(_) => self.jump_to_deleted(&path, one.hit.line, col),
                    None => self.jump_to_col(&path, one.hit.line, col),
                }
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

    /// Whether the cursor stands on the name its line declares, not only on that line (#317):
    /// `let request = session.request(url)` declares the first `request`, and the second is a
    /// method looked up as on any other line. The declared one is the occurrence of `word` the
    /// line no longer reads as a declaration without; a line no pattern reads (a parameter)
    /// declares its first. A word the line does not spell as is (a Ruby setter) is on it, and
    /// so is another occurrence of a shape #317 does not name (see below), unless `exact`: what
    /// implements a member is asked on its declared name alone (#517).
    pub(super) fn on_declared_name(&self, kind: Kind, word: &str, exact: bool) -> bool {
        let line = self.line_str();
        let Some((r, _)) = search::definition_word(Some(kind), line, self.col) else {
            return true;
        };
        let part = |c: char| is_word(c) || search::word_chars(Some(kind), true).contains(c);
        let at: Vec<usize> = line
            .match_indices(word)
            .map(|(i, _)| i)
            .filter(|&i| !line[..i].ends_with(part) && !line[i + word.len()..].starts_with(part))
            .collect();
        if !at.contains(&r.start) {
            return true;
        }
        let mut patterns = search::def_patterns(kind, word);
        patterns.extend(search::member_or_signature(kind, word).unwrap_or_default());
        let re = Regex::new(&patterns.join("|"))
            .ok()
            .filter(|re| re.is_match(line));
        // Another word of the same shape, capitals where it has them: a C# type still reads as one.
        let other: String = word
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
                re.as_ref().is_some_and(|re| {
                    !re.is_match(&format!("{}{other}{}", &line[..i], &line[i + word.len()..]))
                })
            })
            .collect();
        let first = declared.first().copied().unwrap_or(at[0]);
        if declared.contains(&r.start) || (declared.is_empty() && first == r.start) {
            return true;
        }
        if exact {
            return false;
        }
        // Off the declared name, only the shapes #317 is about are looked up as on any other
        // line: a word in front of it (the type of C#'s `Courier Courier`), a member
        // (`session.request`, `$this->chime`), and a call on a line that declares no function
        // (`let chime = chime(total)`). Anything else, a shadowed parameter in `let chime =
        // chime + 1` or the recursion of a one-line `fun fact(n) = … fact(n - 1)`, nothing here
        // tells from another scope's namesake: offered, as on master.
        let called = |i: usize| line[i + word.len()..].trim_start().starts_with('(');
        let before = line[..r.start].trim_end();
        let member = before.ends_with(['.', '>']) || before.ends_with("::");
        !(r.start < first || member || (called(r.start) && !called(first)))
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
        let hits = self.project_grep(kind, here, pattern);
        let mut hits = self.declaring(kind, word, hits);
        // Another file's function locals are not in sight from here (#339).
        if kind == Kind::TsJs {
            let mut lines: HashMap<PathBuf, Vec<String>> = HashMap::new();
            hits.retain(|h| {
                h.path == here
                    || !search::ts_nested_local(
                        lines.entry(h.path.clone()).or_insert_with(|| {
                            self.text_of(&h.path)
                                .map_or_else(Vec::new, |t| t.lines().map(str::to_owned).collect())
                        }),
                        h.line,
                    )
            });
        }
        if kind == Kind::Make {
            let mut recipe = self.make_recipe_rule(Some(here), word);
            hits.retain(|h| recipe(h).unwrap_or(true));
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
        if kind == Kind::Swift {
            self.swift_unseen(here, &mut hits);
        }
        // A Shell `local` belongs to its function: no candidate from any other (#470).
        if kind == Kind::Shell {
            let lines: Vec<&str> = self.buf.lines.iter().map(String::as_str).collect();
            let mine = search::shell_function_at(&lines, self.line);
            hits.retain(|h| {
                let Some(text) = self.text_of(&h.path) else {
                    return true;
                };
                let lines: Vec<&str> = text.lines().collect();
                let at = h.line - 1;
                match lines.get(at).and_then(|l| search::shell_local_of(l)) {
                    Some(_) => search::shell_function_at(&lines, at)
                        .is_none_or(|f| h.path == here && Some(f) == mine),
                    None => true,
                }
            });
        }
        // A common table expression is a declaration only inside its own statement, where it
        // hides a table of its name (#472). On a declaration's own line nothing changes.
        if kind == Kind::Sql
            && !hits
                .iter()
                .any(|h| h.path == here && h.line == self.line + 1)
        {
            let cte = Regex::new(&search::sql_cte(word)).expect("an escaped name keeps it valid");
            let text = self.text_of(here).unwrap_or_default();
            let sees = |h: &Hit| match h.path == here {
                true => search::sql_cte_sees(&text, h.line, word, self.line + 1, self.col),
                false => Some(false),
            };
            // A CTE whose scope has no known end leaves the candidates as they are.
            let known = hits
                .iter()
                .filter(|h| cte.is_match(&h.text))
                .all(|h| sees(h).is_some());
            let seen = hits.iter().any(|h| sees(h) == Some(true));
            hits.retain(|h| match cte.is_match(&h.text) {
                true => !known || sees(h) == Some(true),
                false => !known || !seen,
            });
        }
        hits
    }
}

/// The name `word` is declared by on the line `text`: itself, or for a Java Lombok accessor
/// the field it reads, `title` for `getTitle` (#381).
pub(super) fn declared_as(kind: Kind, word: &str, text: &str) -> String {
    if kind != Kind::Jvm || whole_at(text, word, "").is_some() {
        return word.to_owned();
    }
    search::jvm_accessor(word)
        .and_then(|(names, _)| names.into_iter().find(|n| whole_at(text, n, "").is_some()))
        .unwrap_or_else(|| word.to_owned())
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
