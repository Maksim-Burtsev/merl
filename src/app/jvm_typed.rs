//! `d` in Java and Kotlin through the type a receiver's declaration writes (#388, #391), and the
//! fields Lombok writes accessors for (#381).

use super::*;

/// Where a type written in a Java or Kotlin file is declared.
pub(super) enum JvmType {
    /// The project declares it, on this line.
    Project(Hit),
    /// Nothing in the project declares it: the JDK, a library, the Kotlin standard library.
    Outside(String),
    /// The rules cannot tell: declared more than once, a type parameter, an alias.
    Unknown,
}

impl App {
    /// What `d` answers for `word` behind the receiver `chain` of Java or Kotlin `text`, when the
    /// type of the receiver is read off its declaration: a parameter, a local or a field of the
    /// classes around, then each field of the chain in the type before it. The member of that
    /// type or a type it extends in the project, a Lombok accessor's field or a Kotlin extension
    /// of the project; nothing for a type outside the project, whose members have no source.
    /// `None` leaves the word to the search by name: a link not read, or a type of the project
    /// without the member.
    pub(super) fn jvm_typed(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let kotlin = here.extension().is_none_or(|e| e != "java");
        let Some((first, rest)) = chain.split_first() else {
            let cast = search::jvm_cast_receiver(before, kotlin)?;
            let ty = self.jvm_type_at(here, text, &cast);
            return self.jvm_typed_member(here, ty, cast, word);
        };
        if chain.len() > 6 {
            return None;
        }
        let line = self.line + 1;
        let smart = || kotlin.then(|| search::jvm_smart_cast(text, line, before, first)).flatten();
        let (mut ty, mut links) = match first.as_str() {
            "this" => {
                let lines: Vec<&str> = text.lines().collect();
                let decl = search::jvm_enclosing_types(text, line)
                    .into_iter()
                    .find(|&d| search::jvm_type_name(lines[d - 1]).is_some())?;
                let hit = Hit {
                    deleted: None,
                    path: here.to_path_buf(),
                    line: decl,
                    col: 0,
                    text: lines[decl - 1].to_owned(),
                };
                (JvmType::Project(hit), Vec::new())
            }
            f if f.starts_with(|c: char| c.is_ascii_lowercase()) => {
                let (ty, written) = match smart() {
                    Some(t) => (self.jvm_type_at(here, text, &t), t),
                    None => self.jvm_value(here, text, line, f, kotlin, 1)?,
                };
                (ty, vec![format!("{f}: {written}")])
            }
            _ => return None,
        };
        for (i, field) in rest.iter().enumerate() {
            let JvmType::Project(decl) = ty else {
                return None;
            };
            let (hit, written) = self.jvm_field_type(&decl, field)?;
            let t = self.text_of(&hit.path)?;
            ty = self.jvm_type_at(&hit.path, &t, &written);
            links.push(match (i, first.as_str()) {
                (0, "this") => format!("this.{field}: {written}"),
                _ => format!("{field}: {written}"),
            });
        }
        self.jvm_typed_member(here, ty, links.join(" \u{2192} "), word)
    }

    fn jvm_typed_member(
        &self,
        here: &Path,
        ty: JvmType,
        label: String,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let found = match ty {
            JvmType::Unknown => return None,
            JvmType::Outside(name) => self.jvm_outside(here, word, &name)?,
            JvmType::Project(decl) => {
                let mut names = Vec::new();
                let mut hits = self.jvm_hierarchy_member(&decl, word, 0, &mut names);
                if hits.is_empty() {
                    hits = self.jvm_extensions(here, word);
                    hits.retain(|h| {
                        self.jvm_receiver_of(h, word)
                            .is_some_and(|r| names.contains(&r))
                    });
                }
                if hits.is_empty() {
                    return None;
                }
                hits
            }
        };
        Some(
            found
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect(),
        )
    }

    /// The members `word` of a type `name` outside the project: the project's Kotlin extensions
    /// on it, else none. `None` when the project extends some other type with `word`, which may
    /// be a supertype of `name` (`fun Context.toast` on an `Activity`): by name, as before.
    pub(super) fn jvm_outside(&self, here: &Path, word: &str, name: &str) -> Option<Vec<Hit>> {
        let extensions = self.jvm_extensions(here, word);
        let (on, off): (Vec<Hit>, Vec<Hit>) = extensions
            .into_iter()
            .partition(|h| self.jvm_receiver_of(h, word).as_deref() == Some(name));
        (off.is_empty() || !on.is_empty()).then_some(on)
    }

    /// The Kotlin extensions `fun T.word` and `val T.word` the project declares, on any `T`, and
    /// Scala's `extension (s: T)` methods (#416).
    fn jvm_extensions(&self, here: &Path, word: &str) -> Vec<Hit> {
        let pattern = search::def_patterns(Kind::Jvm, word).join("|");
        let cut = self.truncated.get();
        let mut hits = self.project_definitions(Kind::Jvm, here, word, &pattern);
        self.truncated.set(cut);
        hits.retain(|h| self.jvm_receiver_of(h, word).is_some());
        hits
    }

    /// The receiver type the extension `hit` declares `word` for ([`search::jvm_receiver_at`]).
    fn jvm_receiver_of(&self, hit: &Hit, word: &str) -> Option<String> {
        search::jvm_receiver(&hit.text, word).or_else(|| {
            let text = self.text_of(&hit.path)?;
            search::jvm_receiver_at(&text, hit.line, word)
        })
    }

    /// The type every declaration in scope of the value `name` at 1-based `line` of `text`, the
    /// text of `file`, writes, and where it is declared: its parameters and locals, else the
    /// fields of the classes around it; with `hops`, a local assigned from one call reads the
    /// type the method returns. `None` when one writes none the rules read, two disagree, or a
    /// smart cast, a pattern or a cast may narrow it (`x is T`, `x instanceof T`, `(T) x`).
    fn jvm_value(
        &self,
        file: &Path,
        text: &str,
        line: usize,
        name: &str,
        kotlin: bool,
        hops: usize,
    ) -> Option<(JvmType, String)> {
        let narrowing = match kotlin {
            true => r"\b{0}\s+(?:!?is|as\??)\s|\bwhen\s*\(\s*{0}\s*\)",
            false => r"\b{0}\s+instanceof\s|\(\s*[A-Z][\w.<>]*\s*\)\s*{0}\b",
        };
        let narrowed = Regex::new(&narrowing.replace("{0}", &regex::escape(name))).ok()?;
        if narrowed.is_match(text) {
            return None;
        }
        let lines: Vec<&str> = text.lines().collect();
        let bindings: Vec<usize> = search::bindings(Kind::Jvm, text, line, name)
            .iter()
            .map(|b| b.line)
            .collect();
        let declared = match bindings.is_empty() {
            true => search::jvm_enclosing_types(text, line)
                .into_iter()
                .map(|d| search::jvm_members_of(text, d, name))
                .find(|m| !m.is_empty())?,
            false => bindings,
        };
        let mut found: Option<(JvmType, String)> = None;
        for l in declared {
            let at = lines.get(l - 1)?;
            let this = match search::jvm_declared_type(at, name, kotlin) {
                Some(t) => (self.jvm_type_at(file, text, &t), t),
                None if hops > 0 => {
                    let (recv, call) = search::jvm_assigned_call(at, name)?;
                    self.jvm_returned(file, text, l, recv.as_deref(), &call, kotlin)?
                }
                None => return None,
            };
            if found.as_ref().is_some_and(|(_, w)| *w != this.1) {
                return None;
            }
            found = Some(this);
        }
        found
    }

    /// The type the method `call` returns, called on `recv` (else on the class around 1-based
    /// `line` of `text`, the text of `file`): the one method of the name in that type or a type
    /// it extends in the project, or the field of a Lombok getter.
    fn jvm_returned(
        &self,
        file: &Path,
        text: &str,
        line: usize,
        recv: Option<&str>,
        call: &str,
        kotlin: bool,
    ) -> Option<(JvmType, String)> {
        let owner = match recv {
            Some(r) => self.jvm_value(file, text, line, r, kotlin, 0)?.0,
            None => {
                let lines: Vec<&str> = text.lines().collect();
                let decl = search::jvm_enclosing_types(text, line)
                    .into_iter()
                    .find(|&d| search::jvm_type_name(lines[d - 1]).is_some())?;
                JvmType::Project(Hit {
                    deleted: None,
                    path: file.to_path_buf(),
                    line: decl,
                    col: 0,
                    text: lines[decl - 1].to_owned(),
                })
            }
        };
        let JvmType::Project(decl) = owner else {
            return None;
        };
        let [m] = self
            .jvm_hierarchy_member(&decl, call, 0, &mut Vec::new())
            .try_into()
            .ok()?;
        let mk = m.path.extension().is_none_or(|e| e != "java");
        let written = search::jvm_return_type(&m.text, call, mk).or_else(|| {
            let (names, _) = search::jvm_accessor(call)?;
            names
                .iter()
                .find_map(|n| search::jvm_declared_type(&m.text, n, mk))
        })?;
        let t = self.text_of(&m.path)?;
        Some((self.jvm_type_at(&m.path, &t, &written), written))
    }

    /// The field `field` of the type declared at `decl` or a type it extends in the project, and
    /// the type its declaration writes.
    fn jvm_field_type(&self, decl: &Hit, field: &str) -> Option<(Hit, String)> {
        let mut names = Vec::new();
        let [hit] = self
            .jvm_hierarchy_member(decl, field, 0, &mut names)
            .try_into()
            .ok()?;
        let kotlin = hit.path.extension().is_none_or(|e| e != "java");
        let written = search::jvm_declared_type(&hit.text, field, kotlin)?;
        Some((hit, written))
    }

    /// The declarations of `word` in the type declared at `decl`, else in the types it extends or
    /// implements in the project, nearest first, eight levels deep: its members, else the fields
    /// Lombok writes the accessor `word` for. `names` collects the types walked.
    pub(super) fn jvm_hierarchy_member(
        &self,
        decl: &Hit,
        word: &str,
        depth: usize,
        names: &mut Vec<String>,
    ) -> Vec<Hit> {
        let Some(name) = search::jvm_type_name(&decl.text) else {
            return Vec::new();
        };
        if depth > 8 || names.contains(&name) {
            return Vec::new();
        }
        names.push(name);
        let Some(text) = self.text_of(&decl.path) else {
            return Vec::new();
        };
        let lines: Vec<&str> = text.lines().collect();
        let mut found = search::jvm_members_of(&text, decl.line, word);
        if found.is_empty() {
            found = search::jvm_lombok_fields(&text, decl.line, word);
        }
        found.sort_unstable();
        found.dedup();
        if !found.is_empty() {
            return found
                .into_iter()
                .map(|line| Hit {
                    deleted: None,
                    path: decl.path.clone(),
                    line,
                    col: 0,
                    text: lines[line - 1].to_owned(),
                })
                .collect();
        }
        let mut out = Vec::new();
        for base in search::jvm_bases(&text, decl.line, search::scala(&decl.path)) {
            if let JvmType::Project(b) = self.jvm_type_at(&decl.path, &text, &base) {
                out.extend(self.jvm_hierarchy_member(&b, word, depth + 1, names));
            }
        }
        out
    }

    /// Where the type `written` in `file`, whose text is `text`, is declared: through the import
    /// that binds it, else the one declaration of the project (the file's own first, then its
    /// package's); outside the project when an import names a package the project does not
    /// have, or nothing in the project declares the name at all.
    pub(super) fn jvm_type_at(&self, file: &Path, text: &str, written: &str) -> JvmType {
        if search::jvm_type_parameter(text, written)
            || (search::scala(file) && search::scala_type_parameter(text, written))
        {
            return JvmType::Unknown;
        }
        let is_type = |h: &Hit| search::jvm_type_name(&h.text).as_deref() == Some(written);
        // A declaration-shaped line in a raw string or a text block declares nothing.
        let code = |h: &Hit| {
            self.text_of(&h.path).is_some_and(|t| {
                !search::literal_lines(Kind::Jvm, &t)
                    .get(h.line - 1)
                    .copied()
                    .unwrap_or(false)
            })
        };
        let one = |mut hits: Vec<Hit>| match hits.len() {
            1 => JvmType::Project(hits.remove(0)),
            _ => JvmType::Unknown,
        };
        let imports = search::jvm_imports(text);
        if let Some((_, path)) = imports.iter().find(|(n, _)| n == written) {
            let Some(packages) = self.jvm_packages() else {
                return JvmType::Unknown;
            };
            let split = (1..path.len()).rev().find_map(|n| {
                let files = packages.get(&path[..n].join("."))?;
                Some((files, path[n..].to_vec()))
            });
            let Some((files, names)) = split else {
                return JvmType::Outside(written.to_owned());
            };
            let mut hits: Vec<Hit> = self
                .jvm_declared(files, &names)
                .into_iter()
                .map(|c| c.hit)
                .collect();
            hits.retain(|h| {
                search::jvm_type_name(&h.text).as_deref() == names.last().map(String::as_str)
                    && code(h)
            });
            return one(hits);
        }
        let pattern = search::def_patterns(Kind::Jvm, written).join("|");
        let cut = self.truncated.get();
        let mut hits = self.project_definitions(Kind::Jvm, file, written, &pattern);
        hits.retain(|h| code(h));
        let full = self.truncated.get() && !cut;
        self.truncated.set(cut);
        if full {
            return JvmType::Unknown;
        }
        if hits.is_empty() {
            return JvmType::Outside(written.to_owned());
        }
        let types: Vec<Hit> = hits.iter().filter(|h| is_type(h)).cloned().collect();
        // A Java constructor is declared by its class's name too, in the class's file.
        let others = hits
            .iter()
            .any(|h| !is_type(h) && !types.iter().any(|t| t.path == h.path));
        if others {
            return JvmType::Unknown;
        }
        if types.len() < 2 {
            return one(types);
        }
        let mine: Vec<Hit> = types.iter().filter(|h| h.path == file).cloned().collect();
        if !mine.is_empty() {
            return one(mine);
        }
        let package = search::jvm_package(text);
        let same: Vec<Hit> = types
            .into_iter()
            .filter(|h| {
                package.is_some()
                    && self
                        .text_of(&h.path)
                        .is_some_and(|t| search::jvm_package(&t) == package)
            })
            .collect();
        one(same)
    }

    /// The fields Lombok writes the accessor `word` for, when no method of the name was found
    /// (#381). Behind a qualifier that names a class of the project, `User::getTitle`, those of
    /// that class or a class it extends, proven; else every such field in the project, found by
    /// name, and offered rather than jumped to.
    pub(super) fn jvm_lombok(
        &mut self,
        here: &Path,
        text: &str,
        chain: &[String],
        word: &str,
    ) -> Vec<Candidate> {
        let Some((names, _)) = search::jvm_accessor(word) else {
            return Vec::new();
        };
        if let [owner] = chain
            && owner.starts_with(|c: char| c.is_ascii_uppercase())
        {
            match self.jvm_type_at(here, text, owner) {
                JvmType::Project(decl) => {
                    return self
                        .jvm_hierarchy_member(&decl, word, 0, &mut Vec::new())
                        .into_iter()
                        .map(|hit| Candidate {
                            hit,
                            reason: Reason::Path(owner.clone()),
                        })
                        .collect();
                }
                JvmType::Outside(_) => return Vec::new(),
                JvmType::Unknown => {}
            }
        }
        let mut found = Vec::new();
        for name in &names {
            let pattern = search::def_patterns(Kind::Jvm, name).join("|");
            for h in self.project_definitions(Kind::Jvm, here, name, &pattern) {
                if h.path.extension().is_none_or(|e| e != "java") {
                    continue;
                }
                let Some(t) = self.text_of(&h.path) else {
                    continue;
                };
                let Some(&decl) = search::jvm_enclosing_types(&t, h.line).first() else {
                    continue;
                };
                if search::jvm_lombok_fields(&t, decl, word).contains(&h.line) {
                    found.push(Candidate {
                        hit: h,
                        reason: Reason::ByName,
                    });
                }
            }
        }
        self.offer_only |= !found.is_empty();
        found
    }
}
