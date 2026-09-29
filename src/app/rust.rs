//! `d` in Rust, what the shared lookup cannot read off a line: the words of an attribute, a
//! field access, an enum variant (#370).

use super::*;

impl App {
    /// What `d` on `word` at `range` of the cursor's line answers before any other lookup, in a
    /// Rust file: `None` leaves the word to the rest of [`App::goto_definition`].
    ///
    /// - A word inside an attribute is a macro or nothing ([`search::rust_attribute`]): a derive
    ///   is a `pub macro W` or a `#[proc_macro_derive(W` line, an attribute's own name a `pub
    ///   macro W` or a `pub fn W` under `#[proc_macro_attribute]`, and a `cfg` predicate or a
    ///   compiler attribute is declared nowhere. No project `struct`, `fn`, `let` or `mod` is one.
    /// - `x.w` with no `(` or `::<` behind it is a field: Rust never calls a field and never
    ///   reads a method without its `()`.
    /// - An enum variant's own line is a declaration, answered as any other.
    pub(super) fn rust_early(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        range: std::ops::Range<usize>,
        dotted: bool,
    ) -> Option<Vec<Candidate>> {
        let kind = Kind::Rust;
        let after = &self.line_str()[range.end..];
        if let Some(attr) =
            search::rust_attribute(&self.buf.lines, self.line, range.start, range.end)
        {
            return Some(match attr {
                search::RustAttr::Nothing => Vec::new(),
                _ => self.rust_macros(attr, word),
            });
        }
        let named = word.starts_with(|c: char| c.is_alphabetic() || c == '_');
        if dotted && named && word != "await" && !after.starts_with('(') && !after.starts_with("::")
        {
            return Some(self.rust_fields(here, word));
        }
        // On the variant's own name, behind its attributes at most.
        let before = self.line_str()[..range.start].trim();
        let variant = (before.is_empty() || (before.starts_with("#[") && before.ends_with(']')))
            && Regex::new(&search::rust_variant_pattern(word))
                .is_ok_and(|re| re.is_match(self.line_str()))
            && search::rust_variant_at(text, self.line + 1);
        if variant {
            let pattern = search::def_patterns(kind, word).join("|");
            let mut found: Vec<Candidate> = self
                .project_definitions(kind, here, word, &pattern)
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect();
            found.push(Candidate {
                hit: Hit {
                    path: here.to_path_buf(),
                    line: self.line + 1,
                    col: 0,
                    text: self.line_str().to_owned(),
                },
                reason: Reason::ByName,
            });
            return Some(found);
        }
        None
    }

    /// The macros a word of an attribute names, outside the project and, for a proc-macro
    /// crate of its own, in it.
    fn rust_macros(&mut self, attr: search::RustAttr, word: &str) -> Vec<Candidate> {
        let kind = Kind::Rust;
        let patterns = search::rust_macro_patterns(attr, word);
        let mut hits = self
            .grep(&patterns[1..].join("|"), false, false, |p| {
                p.extension().is_some_and(|e| e == "rs")
            })
            .unwrap_or_default();
        let files = self.external_files(kind);
        hits.extend(self.external_grep(kind, &files, &patterns.join("|")));
        hits.into_iter()
            .filter(|h| {
                self.text_of(&h.path)
                    .is_some_and(|t| search::rust_proc_macro(&t, h.line))
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    /// The fields `word` a `x.word` can read: the project's, one row per struct, else the `pub`
    /// ones of the standard library and the dependencies. A private field outside the project
    /// cannot be reached.
    fn rust_fields(&mut self, here: &Path, word: &str) -> Vec<Candidate> {
        let kind = Kind::Rust;
        let field = |this: &Self, h: &Hit| {
            this.text_of(&h.path)
                .is_some_and(|t| search::rust_field_at(&t, h.line))
        };
        let mut hits =
            self.project_definitions(kind, here, word, &search::rust_field_pattern(word, false));
        hits.retain(|h| field(self, h));
        if hits.is_empty() {
            let files = self.external_files(kind);
            hits = self.external_grep(kind, &files, &search::rust_field_pattern(word, true));
            hits.retain(|h| field(self, h));
        }
        hits.into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    /// The project's lines that declare the enum variant `word`. Rust names a variant in CamelCase, and a lowercase word is not worth the grep.
    pub(super) fn rust_variants(&self, here: &Path, word: &str) -> Vec<Hit> {
        if !word.starts_with(|c: char| c.is_ascii_uppercase()) {
            return Vec::new();
        }
        let mut hits = self
            .grep(&search::rust_variant_pattern(word), false, false, |p| {
                search::in_def_scope(Kind::Rust, here, p)
            })
            .unwrap_or_default();
        hits.retain(|h| {
            self.text_of(&h.path)
                .is_some_and(|t| search::rust_variant_at(&t, h.line))
        });
        hits
    }

    /// A bare `word` behind a `use …::E::*;` in scope ([`search::rust_glob_uses`]) is the
    /// variant `word` of `E`, when `E` declares one: the enum this file declares, else every
    /// enum of the project so called. A column-zero glob gives way to a `use` that names the
    /// word and to an item of the file so called; the function's own glob hides both.
    pub(super) fn rust_glob_variant(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        imports: &[(String, Vec<String>)],
    ) -> Vec<Candidate> {
        let kind = Kind::Rust;
        let (globs, own) = search::rust_glob_uses(text, self.line + 1);
        if globs.is_empty() {
            return Vec::new();
        }
        let top_level = || {
            let item = Regex::new(&search::def_patterns(kind, word)[..2].join("|"))
                .expect("an escaped name keeps the pattern valid");
            text.lines()
                .any(|l| !l.starts_with([' ', '\t']) && item.is_match(l))
        };
        if !own && (bound(imports, word).is_some() || top_level()) {
            return Vec::new();
        }
        let variants = self.rust_variants(here, word);
        let mut found = Vec::new();
        for glob in globs {
            let Some(owner) = glob.last() else { continue };
            let enums = self
                .project_definitions(kind, here, owner, &search::def_patterns(kind, owner)[0])
                .into_iter()
                .filter(|h| h.text.contains("enum"))
                .collect::<Vec<_>>();
            let mine = enums.iter().any(|h| h.path == here);
            for v in &variants {
                let Some(t) = self.text_of(&v.path) else {
                    continue;
                };
                let full = format!("{owner}::{word}");
                let of = search::qualified(kind, &t, v.line, word)
                    .is_some_and(|q| q == full || q.ends_with(&format!("::{full}")));
                let in_enum = of
                    && enums
                        .iter()
                        .any(|e| e.path == v.path && (!mine || e.path == here) && e.line < v.line);
                if in_enum
                    && !found
                        .iter()
                        .any(|c: &Candidate| (&c.hit.path, c.hit.line) == (&v.path, v.line))
                {
                    found.push(Candidate {
                        hit: v.clone(),
                        reason: Reason::Import(format!("{}::*", glob.join("::"))),
                    });
                }
            }
        }
        found
    }

    /// A bare `word` at `range` of the cursor's line is the item this file declares where the
    /// cursor sees it ([`search::rust_scope_items`], #363). A struct literal's or a pattern's
    /// `word:` names a field, no item.
    pub(super) fn rust_file_items(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        range: std::ops::Range<usize>,
    ) -> Vec<Candidate> {
        let after = self.line_str()[range.end..].trim_start();
        if after.starts_with(':') && !after.starts_with("::") {
            return Vec::new();
        }
        let ns = if after.starts_with('!') && !after.starts_with("!=") {
            search::RustNamespace::Macro
        } else if after.starts_with("::") {
            search::RustNamespace::Path
        } else {
            search::RustNamespace::Other
        };
        search::rust_scope_items(text, self.line + 1, word, ns)
            .into_iter()
            .map(|line| Candidate {
                hit: Hit {
                    path: here.to_path_buf(),
                    line,
                    col: 0,
                    text: self.buf.lines[line - 1].clone(),
                },
                reason: Reason::File,
            })
            .collect()
    }

    /// `x.word(…)` on a value whose type is not known (#358): the methods `word` of the project,
    /// the standard library and the dependencies, each an indented `fn word` directly in an
    /// `impl` or a `trait` ([`search::rust_method_at`]), less what the cursor cannot reach: an
    /// inherent method without `pub` outside its module (its file and the directory below it),
    /// a `pub(crate)` one outside its crate (the directory of its `Cargo.toml`), and a crate the
    /// cursor's does not reach through `Cargo.lock`. The traits' own methods come first. When
    /// every candidate is the method of one trait or of an `impl` of it, that method is where
    /// every call lands, `via trait Tr`.
    pub(super) fn rust_methods(&mut self, here: &Path, word: &str) -> Vec<Candidate> {
        let kind = Kind::Rust;
        let pattern = search::rust_method_pattern(word);
        let mut hits = self.project_definitions(kind, here, word, &pattern);
        let files = self.external_files(kind);
        hits.extend(self.external_grep(kind, &files, &pattern));
        let crate_of = |p: &Path| {
            p.ancestors()
                .skip(1)
                .find(|d| self.files.contains(&d.join("Cargo.toml")))
                .map(Path::to_path_buf)
        };
        let package = |dir: &Path| {
            self.text_of(&dir.join("Cargo.toml"))
                .and_then(|t| search::cargo_package_name(&t))
        };
        let own = crate_of(here);
        let lock = std::fs::read_to_string(self.root.join("Cargo.lock")).unwrap_or_default();
        let reach = own
            .as_deref()
            .and_then(package)
            .and_then(|name| search::cargo_reach(&lock, &name));
        // A crate the lock does not list, the standard library's, is always reached. A registry
        // copy of a crate of the workspace is not the one built.
        let reached = |h: &Hit| {
            let Some((reached, dirs)) = &reach else {
                return true;
            };
            let name = match h.path.is_absolute() {
                true => {
                    let listed = h.path.ancestors().find_map(|a| {
                        let dir = a.file_name()?.to_str()?;
                        dirs.iter().find(|(d, _, _)| d == dir)
                    });
                    match listed {
                        Some((_, _, false)) => return false,
                        found => found.map(|(_, n, _)| n.clone()),
                    }
                }
                false => crate_of(&h.path).as_deref().and_then(package),
            };
            name.is_none_or(|n| reached.contains(&n) || !dirs.iter().any(|(_, m, _)| *m == n))
        };
        // A module's private items are its own and its children's: `src/foo.rs` has
        // `src/foo/`, `lib.rs`, `main.rs` and `mod.rs` their directory.
        let below = |p: &Path| {
            let root_file = p
                .file_stem()
                .is_some_and(|s| s == "lib" || s == "main" || s == "mod");
            let dir = match root_file {
                true => p.parent().unwrap_or(Path::new("")).to_path_buf(),
                false => p.with_extension(""),
            };
            here == p || here.starts_with(dir)
        };
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.clone())
            .unwrap_or_default();
        let mut texts: HashMap<PathBuf, String> = HashMap::new();
        let mut rows: Vec<(Hit, search::RustOwner)> = Vec::new();
        for h in hits {
            if !texts.contains_key(&h.path) {
                let text = self.text_of(&h.path).unwrap_or_default();
                texts.insert(h.path.clone(), text);
            }
            let lines: Vec<&str> = texts[&h.path].lines().collect();
            let Some((owner, vis)) = search::rust_method_at(&lines, h.line) else {
                continue;
            };
            let inside = !h.path.is_absolute();
            // Outside, a crate's tests, benches and examples are crates of their own, and a
            // method of the standard library with no stability attribute is internal to it.
            let rel = roots
                .iter()
                .find_map(|r| h.path.strip_prefix(r).ok())
                .unwrap_or(&h.path);
            let internal = !inside
                && (rel.components().any(|c| {
                    matches!(
                        c.as_os_str().to_str(),
                        Some("tests" | "benches" | "examples")
                    )
                }) || (matches!(
                    owner,
                    search::RustOwner::Inherent | search::RustOwner::Unreadable
                ) && h
                    .path
                    .to_string_lossy()
                    .contains("rustlib/src/rust/library/")
                    && !search::rust_stability(&lines, h.line)));
            let visible = !internal
                && match (&owner, vis) {
                    (search::RustOwner::Trait(_) | search::RustOwner::ImplOf(_), _) => true,
                    (_, search::RustVis::Pub) => true,
                    (_, search::RustVis::Crate) => inside && crate_of(&h.path) == own,
                    (search::RustOwner::Inherent, search::RustVis::Private) => {
                        inside && below(&h.path)
                    }
                    (_, search::RustVis::Private) => true,
                };
            if visible && reached(&h) {
                rows.push((h, owner));
            }
        }
        rows.sort_by_key(|(_, o)| !matches!(o, search::RustOwner::Trait(_)));
        let traits: Vec<&String> = rows
            .iter()
            .filter_map(|(_, o)| match o {
                search::RustOwner::Trait(t) => Some(t),
                _ => None,
            })
            .collect();
        if let [tr] = traits.as_slice()
            && rows.iter().all(|(_, o)| {
                matches!(o, search::RustOwner::Trait(t) | search::RustOwner::ImplOf(t) if t == *tr)
            })
        {
            let reason = Reason::Trait((*tr).clone());
            return vec![Candidate {
                hit: rows.swap_remove(0).0,
                reason,
            }];
        }
        rows.into_iter()
            .map(|(hit, _)| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }
}
