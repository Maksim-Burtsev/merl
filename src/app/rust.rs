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
                    deleted: None,
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
                    deleted: None,
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
        // Behind a `.` the word is never the declaration on its own line: a one-line
        // `fn is_empty(&self) -> bool { self.0.is_empty() }` calls another.
        hits.retain(|h| h.path != here || h.line != self.line + 1);
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
        // The packages the project's own `Cargo.toml`s declare: a registry copy of one of them
        // is not the one built.
        let workspace: Vec<String> = self
            .files
            .iter()
            .filter(|f| f.file_name().is_some_and(|n| n == "Cargo.toml"))
            .filter_map(|f| f.parent().and_then(package))
            .collect();
        // A crate the lock does not list, the standard library's, is always reached.
        let reached = |h: &Hit| {
            let name = match h.path.is_absolute() {
                true => {
                    let listed = h.path.ancestors().find_map(|a| {
                        let dir = a.file_name()?.to_str()?;
                        let dirs = reach
                            .as_ref()
                            .map(|(_, d)| d.as_slice())
                            .unwrap_or_default();
                        dirs.iter().find(|(d, _)| d == dir).map(|(_, n)| n.clone())
                    });
                    if listed.as_ref().is_some_and(|n| workspace.contains(n)) {
                        return false;
                    }
                    listed
                }
                false => crate_of(&h.path).as_deref().and_then(package),
            };
            let Some((reached, dirs)) = &reach else {
                return true;
            };
            name.is_none_or(|n| reached.contains(&n) || !dirs.iter().any(|(_, m)| *m == n))
        };
        // A module's private items are its own and its children's: `src/foo.rs` has `src/foo/`.
        // A crate root has its directory: `lib.rs`, `main.rs`, `mod.rs`, `build.rs`, and a file
        // directly in `tests/`, `benches/`, `examples/` or `src/bin/`.
        let below = |p: &Path| {
            let parent = p.parent().unwrap_or(Path::new(""));
            let root_file = p
                .file_stem()
                .is_some_and(|s| s == "lib" || s == "main" || s == "mod" || s == "build")
                || ["tests", "benches", "examples"]
                    .iter()
                    .any(|d| parent.file_name().is_some_and(|n| n == *d))
                || parent.ends_with("src/bin");
            let dir = match root_file {
                true => parent.to_path_buf(),
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
        let mut hidden: Vec<String> = Vec::new();
        for h in hits {
            if !texts.contains_key(&h.path) {
                let text = self.text_of(&h.path).unwrap_or_default();
                texts.insert(h.path.clone(), text);
            }
            let lines: Vec<&str> = texts[&h.path].lines().collect();
            let Some((owner, vis, owner_line)) = search::rust_method_at(&lines, h.line) else {
                continue;
            };
            let inside = !h.path.is_absolute();
            // Outside, a crate's tests, benches and examples are crates of their own, and a
            // method of the standard library with no stability attribute is internal to it.
            let rel = roots
                .iter()
                .find_map(|r| h.path.strip_prefix(r).ok())
                .unwrap_or(&h.path);
            // The standard library marks what it offers `#[stable]` or `#[unstable]`: an inherent
            // method, or a trait, with neither is its own (compiler-builtins' `Int`, a vendored
            // `gimli`). A method a macro writes has the attributes the macro is handed.
            let sysroot = h
                .path
                .to_string_lossy()
                .contains("rustlib/src/rust/library/");
            let unmarked = sysroot
                && match owner {
                    search::RustOwner::Inherent => !search::rust_stability(&lines, h.line),
                    search::RustOwner::Trait(_) => !search::rust_stability(&lines, owner_line),
                    _ => false,
                };
            let internal = !inside
                && (unmarked
                    || rel.components().any(|c| {
                        matches!(
                            c.as_os_str().to_str(),
                            Some("tests" | "benches" | "examples")
                        )
                    }));
            let visible = !internal
                && match (&owner, vis) {
                    (search::RustOwner::ImplOf(_), _) => true,
                    // A trait outside the project that is not `pub` is its crate's own.
                    (search::RustOwner::Trait(_), search::RustVis::Pub) => true,
                    (search::RustOwner::Trait(_), _) => inside,
                    (_, search::RustVis::Pub) => true,
                    (_, search::RustVis::Crate) => inside && crate_of(&h.path) == own,
                    (search::RustOwner::Inherent, search::RustVis::Private) => {
                        inside && below(&h.path)
                    }
                    (_, search::RustVis::Private) => true,
                };
            if let (search::RustOwner::Trait(t), false) = (&owner, visible) {
                hidden.push(t.clone());
            }
            if visible && reached(&h) {
                rows.push((h, owner));
            }
        }
        // The `impl`s outside the project of a trait out of reach are as far out of reach.
        let kept: Vec<String> = rows
            .iter()
            .filter_map(|(_, o)| match o {
                search::RustOwner::Trait(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        rows.retain(|(h, o)| {
            !matches!(o, search::RustOwner::ImplOf(t)
                if h.path.is_absolute() && hidden.contains(t) && !kept.contains(t))
        });
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

    /// Where a Rust path is declared, by the crate its first name names (#350): a `use` of the
    /// file binds that name, or it is written out (`std::fs::File::create`). `crate`, `self`
    /// and `super` are the project's crate and module, a `[package]` or `[lib]` name of a
    /// `Cargo.toml` of the project is that crate, `std`, `core`, `alloc` and `proc_macro` are
    /// the sysroot's, any other name a crate of `Cargo.lock` in the registry. In the crate, the
    /// longest module the path spells, `a::b` being `src/a/b.rs` or `src/a/b/mod.rs`, is read
    /// first, then the files under its directory, then the whole crate (a `pub use` from
    /// elsewhere); a name the path takes further through a type, `File::open`, is kept only
    /// where [`search::qualified`] names it so. What `std` does not declare it hands on from
    /// `core` or `alloc`. A primitive in front, `usize::MAX`, names an item of `core`, by name.
    /// `None` when no crate is named or nothing is found there: the lookup goes on as before.
    pub(super) fn rust_crate_path(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        chain: &[String],
        macro_call: Option<bool>,
    ) -> Option<Vec<Candidate>> {
        const PRIMITIVES: &[&str] = &[
            "bool", "char", "str", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32",
            "i64", "i128", "isize", "f32", "f64",
        ];
        const SYSROOT: &[&str] = &["std", "core", "alloc", "proc_macro"];
        let first = chain.first().map_or(word, String::as_str);
        // A `use` in column zero binds the whole file; an indented one, in a function or an
        // inline `mod`, its block: the lines below it indented as far.
        let lines: Vec<&str> = text.lines().collect();
        let indent = |l: &str| l.len() - l.trim_start().len();
        let sees = |at: usize| {
            let from = text[..at].matches('\n').count();
            let depth = indent(lines[from]);
            depth == 0
                || (from < self.line
                    && lines[from + 1..=self.line]
                        .iter()
                        .all(|l| l.trim().is_empty() || indent(l) >= depth))
        };
        let depth_at = |at: usize| indent(lines[text[..at].matches('\n').count()]);
        let mut bound: Vec<(Vec<String>, usize)> = search::rust_uses_at(text)
            .into_iter()
            .filter(|(n, _, at)| n == first && sees(*at))
            .map(|(_, p, at)| (p, depth_at(at)))
            .collect();
        bound.dedup();
        // A glob `use` of a block nearer the cursor than the `use` of the name may bring in a
        // name of its own that hides it (`use self::Strategy::*;` then `Regex(ref s) =>`): a
        // bare name is left to the lookup after this one then.
        let glob = Regex::new(r"(?m)^[ \t]+use\s[^;]*\*\s*;").expect("a valid pattern");
        if let [(_, depth)] = bound.as_slice()
            && chain.is_empty()
            && glob
                .find_iter(text)
                .any(|m| sees(m.start()) && depth_at(m.start()) > *depth)
        {
            return None;
        }
        let (mut full, imported) = match bound.as_slice() {
            [(p, depth)] if *depth == 0 || !matches!(p[0].as_str(), "self" | "super") => {
                (p.clone(), true)
            }
            [] if !chain.is_empty() => (vec![first.to_owned()], false),
            _ => return None,
        };
        if !chain.is_empty() {
            full.extend(chain[1..].iter().cloned());
            full.push(word.to_owned());
        }
        let (root, rest) = full.split_first()?;
        if rest.is_empty() {
            return None;
        }
        // A module of this file's own, `mod log {` or `mod log;`, is no crate `log`.
        let own_mod = Regex::new(&format!(
            r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+{}\b",
            regex::escape(root)
        ))
        .is_ok_and(|re| re.is_match(text));
        let kind = Kind::Rust;
        let by = |hits: Vec<Hit>, reason: Reason| -> Vec<Candidate> {
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect()
        };
        if PRIMITIVES.contains(&root.as_str()) && !imported {
            let files = self.sysroot_crate("core")?.1;
            let name = rest.last()?;
            let pattern = search::def_patterns(kind, name).join("|");
            let hits = search::grep_project(&self.root, &files, &pattern, false, false, None, None)
                .unwrap_or_default();
            self.note_cut(&hits);
            let hits = self.declaring(kind, name, hits);
            return (!hits.is_empty()).then(|| by(hits, Reason::ByName));
        }
        // The crate's `src/`, its files, and the module the path starts in.
        let (src, files, base, name) = match root.as_str() {
            "crate" | "self" | "super" => {
                let (src, mut module) = search::rust_module_of(&self.files, here)?;
                match root.as_str() {
                    "crate" => module.clear(),
                    "super" => {
                        let supers = 1 + rest.iter().take_while(|p| *p == "super").count();
                        module.truncate(module.len().checked_sub(supers)?);
                    }
                    _ => {}
                }
                let files: Vec<PathBuf> = self
                    .files
                    .iter()
                    .filter(|f| f.starts_with(&src))
                    .cloned()
                    .collect();
                (src, files, module, "crate".to_owned())
            }
            _ if own_mod => return None,
            r if SYSROOT.contains(&r) => {
                let (src, files) = self.sysroot_crate(r)?;
                (src, files, Vec::new(), root.clone())
            }
            r => {
                let (src, files) = self.workspace_crate(r).or_else(|| self.registry_crate(r))?;
                (src, files, Vec::new(), root.clone())
            }
        };
        let rest: Vec<String> = rest.iter().filter(|p| *p != "super").cloned().collect();
        let sep = "::";
        // A name the crate declares only somewhere else than the path says is found by name.
        let reason = |crate_name: &str, module: Vec<String>, proven: bool| {
            let mut shown = vec![crate_name.to_owned()];
            shown.extend(module);
            let spelled = shown.join(sep);
            match (proven, imported) {
                (false, _) => Reason::ByName,
                (true, true) => Reason::Import(spelled),
                (true, false) => Reason::Path(spelled),
            }
        };
        let found = self.rust_in_crate(&src, &files, &base, &rest, macro_call, 0);
        if let Some((module, hits, proven)) = found {
            return Some(by(hits, reason(&name, module, proven)));
        }
        // `std` hands on what `core` and `alloc` declare: `std::sync::Arc` is `alloc`'s.
        if root == "std" {
            for other in ["core", "alloc"] {
                let Some((src, files)) = self.sysroot_crate(other) else {
                    continue;
                };
                if let Some((module, hits, proven)) =
                    self.rust_in_crate(&src, &files, &[], &rest, macro_call, 0)
                {
                    return Some(by(hits, reason(other, module, proven)));
                }
            }
        }
        None
    }

    /// The declarations of the last name of `path` in the crate at `src` of `files`, from its
    /// module `base`, with the module that held them and whether the path proves them: in the
    /// longest module `path` spells, then where a `use` there takes the name from in the crate
    /// (`pub use crate::store::Cache`), then under the module's directory; then, for a name the
    /// path takes through a type (`Cache::new`), in the whole crate, found by name.
    fn rust_in_crate(
        &self,
        src: &Path,
        files: &[PathBuf],
        base: &[String],
        path: &[String],
        macro_call: Option<bool>,
        depth: usize,
    ) -> Option<(Vec<String>, Vec<Hit>, bool)> {
        let (name, parts) = path.split_last()?;
        let mut module: Vec<String> = base.to_vec();
        module.extend(parts.iter().cloned());
        let file_of = |m: &[String]| -> Vec<PathBuf> {
            let at = m.iter().fold(src.to_path_buf(), |p, n| p.join(n));
            let wanted = match m.is_empty() {
                true => vec![src.join("lib.rs"), src.join("main.rs")],
                false => vec![at.with_extension("rs"), at.join("mod.rs")],
            };
            wanted.into_iter().filter(|f| files.contains(f)).collect()
        };
        let n = (base.len()..=module.len())
            .rev()
            .find(|&n| !file_of(&module[..n]).is_empty())?;
        let (at, inside) = module.split_at(n);
        let owner = (!inside.is_empty()).then(|| inside.join("::"));
        let here = file_of(at);
        let hits = self.rust_declared(&here, name, owner.as_deref(), macro_call);
        if !hits.is_empty() {
            return Some((at.to_vec(), hits, true));
        }
        // A `use` of the module's file that binds the first name left, the crate's own.
        let first = inside.first().unwrap_or(name);
        let taken = here.iter().find_map(|f| {
            let text = self.text_of(f)?;
            let mut bound = search::rust_uses(&text)
                .into_iter()
                .filter(|(n, _, top)| n == first && *top)
                .map(|(_, p, _)| p);
            let p = bound.next()?;
            let to: Vec<String> = match p.first()?.as_str() {
                "crate" => p[1..].to_vec(),
                "super" => {
                    let up = p.iter().take_while(|s| *s == "super").count();
                    let kept = at.len().checked_sub(up)?;
                    at[..kept].iter().chain(&p[up..]).cloned().collect()
                }
                // `self::` is dropped from what a `use` binds: a module below this one.
                _ => at.iter().chain(&p).cloned().collect(),
            };
            Some(to)
        });
        if let Some(mut to) = taken.filter(|_| depth < 4) {
            to.extend(inside.iter().skip(1).cloned());
            if !inside.is_empty() {
                to.push(name.clone());
            }
            if let Some(found) = self.rust_in_crate(src, files, &[], &to, macro_call, depth + 1) {
                return Some(found);
            }
        }
        let dir = at.iter().fold(src.to_path_buf(), |p, m| p.join(m));
        let under: Vec<PathBuf> = files
            .iter()
            .filter(|f| f.starts_with(&dir))
            .cloned()
            .collect();
        let hits = self.rust_declared(&under, name, owner.as_deref(), macro_call);
        if !hits.is_empty() {
            return Some((at.to_vec(), hits, true));
        }
        // ponytail: a name at the top of a module found nowhere else is left to the search
        // by name; the whole crate would offer another module's namesake as proven.
        let hits = match owner {
            Some(_) => self.rust_declared(files, name, owner.as_deref(), macro_call),
            None => Vec::new(),
        };
        (!hits.is_empty()).then(|| (at.to_vec(), hits, false))
    }

    /// The lines of `files` that declare `name` inside `owner` as [`search::qualified`] names
    /// it (`File` for `File::open`), or at the top of their module with no `owner`. A macro
    /// lives apart from everything else: `Some(true)` keeps the `macro_rules!` alone, `Some(false)`
    /// all but them.
    fn rust_declared(
        &self,
        files: &[PathBuf],
        name: &str,
        owner: Option<&str>,
        macro_call: Option<bool>,
    ) -> Vec<Hit> {
        let kind = Kind::Rust;
        let pattern = search::def_patterns(kind, name).join("|");
        let hits = search::grep_project(&self.root, files, &pattern, false, false, None, None)
            .unwrap_or_default();
        self.note_cut(&hits);
        let want = owner.map(|o| format!("{o}::{name}"));
        let mut hits = self.declaring(kind, name, hits);
        hits.retain(|h| {
            macro_call.is_none_or(|m| m == h.text.trim_start().starts_with("macro_rules!"))
                && self
                    .text_of(&h.path)
                    .map(|t| search::qualified(kind, &t, h.line, name))
                    == Some(want.clone())
        });
        hits
    }

    /// The `src/` of the sysroot's crate `name` (`std`, `core`…) and its files.
    fn sysroot_crate(&mut self, name: &str) -> Option<(PathBuf, Vec<PathBuf>)> {
        let all = self.external_files(Kind::Rust);
        let roots = self.external.get(&Kind::Rust)?.0.clone();
        roots.iter().find_map(|r| {
            let src = r.join(name).join("src");
            let files: Vec<PathBuf> = all
                .iter()
                .filter(|f| f.starts_with(&src))
                .cloned()
                .collect();
            (!files.is_empty()).then_some((src, files))
        })
    }

    /// The `src/` of the registry crate a path calls `name`, a directory `name-1.2.3` with `_`
    /// spelled `-`, and its files.
    // ponytail: the first version `Cargo.lock` holds; two versions of one crate are not told apart.
    fn registry_crate(&mut self, name: &str) -> Option<(PathBuf, Vec<PathBuf>)> {
        let all = self.external_files(Kind::Rust);
        let roots = self.external.get(&Kind::Rust)?.0.clone();
        roots.iter().find_map(|r| {
            let dir = r.file_name()?.to_string_lossy().into_owned();
            let (i, _) = dir
                .rmatch_indices('-')
                .find(|(i, _)| dir[i + 1..].starts_with(|c: char| c.is_ascii_digit()))?;
            if dir[..i].replace('-', "_") != name {
                return None;
            }
            let src = r.join("src");
            let files: Vec<PathBuf> = all
                .iter()
                .filter(|f| f.starts_with(&src))
                .cloned()
                .collect();
            (!files.is_empty()).then_some((src, files))
        })
    }

    /// The `src/` of the crate of the project whose `Cargo.toml` names it `name`, as its
    /// `[package]` or its `[lib]`, with `-` read as `_`, and its files.
    fn workspace_crate(&self, name: &str) -> Option<(PathBuf, Vec<PathBuf>)> {
        self.files
            .iter()
            .filter(|f| f.file_name().is_some_and(|n| n == "Cargo.toml"))
            .find_map(|toml| {
                let text = self.text_of(toml)?;
                let mut section = "";
                let named = text.lines().any(|l| {
                    let l = l.trim();
                    if l.starts_with('[') {
                        section = l;
                        return false;
                    }
                    let value = l
                        .strip_prefix("name")
                        .map(str::trim_start)
                        .and_then(|v| v.strip_prefix('='))
                        .map(|v| v.trim().trim_matches('"').replace('-', "_"));
                    matches!(section, "[package]" | "[lib]") && value.as_deref() == Some(name)
                });
                let src = toml.parent()?.join("src");
                let files: Vec<PathBuf> = self
                    .files
                    .iter()
                    .filter(|f| f.starts_with(&src))
                    .cloned()
                    .collect();
                (named && !files.is_empty()).then_some((src, files))
            })
    }

    /// `d` on `x.word` in Rust when the type of `x` is proven (#377): `self` is the type of the
    /// `impl` around the method, a parameter or a `let` says what it holds by its written type, a
    /// struct literal, a call of an associated function of `T` that returns `Self` or `T`, or a
    /// call of a function the file sees declared once with its `-> T`. Each name of the chain
    /// after it is a field of the struct before it, up to six names. The word is a method of the
    /// last type (`call`), in the `impl T` and `impl Tr for T` blocks of its crate, else a method
    /// with a body in a trait it implements; or its field. `Err` names the first name that is not
    /// proven, an empty list a type without the word: both leave the word to the search by name.
    pub(super) fn rust_typed(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        chain: &[String],
        call: bool,
    ) -> Result<Vec<Candidate>, String> {
        // ponytail: six names in front of the word; a longer chain breaks at the seventh.
        if let Some(seventh) = chain.get(6) {
            return Err(seventh.clone());
        }
        let (mut ty, link) = self
            .rust_value_type(here, text, self.line, &chain[0])
            .ok_or_else(|| chain[0].clone())?;
        let mut links = vec![link];
        for (i, field) in chain.iter().enumerate().skip(1) {
            let next = self
                .rust_field(&ty, field)
                .and_then(|(hit, written)| self.rust_resolve(&ty.path, &written, hit.line - 1))
                .ok_or_else(|| field.clone())?;
            match i {
                1 => links[0] = format!("{}.{field}: {}", chain[0], next.name),
                _ => links.push(format!("{field}: {}", next.name)),
            }
            ty = next;
        }
        let hits = match call {
            true => self.rust_members(&ty, word),
            false => self
                .rust_field(&ty, word)
                .map(|(hit, _)| vec![hit])
                .unwrap_or_default(),
        };
        let label = links.join(" \u{2192} ");
        Ok(hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Receiver(label.clone()),
            })
            .collect())
    }

    /// The type of the local, parameter or `self` `name` on 0-based line `at` of `text`, the text
    /// of `file`, and the link that proves it: every binding in scope reads the same type.
    fn rust_value_type(
        &self,
        file: &Path,
        text: &str,
        at: usize,
        name: &str,
    ) -> Option<(Typed, String)> {
        let lines: Vec<&str> = text.lines().collect();
        if name == "self" {
            let (written, impl_line) = search::rust_self_type(&lines, at)?;
            let ty = self.rust_resolve(file, &written, impl_line)?;
            let link = format!("self: {}", ty.name);
            return Some((ty, link));
        }
        let bindings = search::bindings(Kind::Rust, text, at + 1, name);
        let mut found: Option<(Typed, String)> = None;
        for b in &bindings {
            let at = b.line - 1;
            let this = match search::rust_holds(&lines, at, name)? {
                search::RustHolds::Type(w) | search::RustHolds::Literal(w) => {
                    let ty = self.rust_resolve(file, &w, at)?;
                    let link = format!("{name}: {}", ty.name);
                    (ty, link)
                }
                search::RustHolds::Assoc(w, f) => {
                    let ty = self.rust_resolve(file, &w, at)?;
                    // `T::default()` is `Default`'s, which a derive declares out of sight.
                    let returns = match <[Hit; 1]>::try_from(self.rust_members(&ty, &f)) {
                        Ok([decl]) => {
                            let t = self.text_of(&decl.path)?;
                            let l: Vec<&str> = t.lines().collect();
                            let (_, n) = search::rust_type_name(&search::rust_return_type(
                                &l,
                                decl.line - 1,
                            )?)?;
                            n == "Self" || n == ty.name
                        }
                        Err(none) => none.is_empty() && f == "default",
                    };
                    if !returns {
                        return None;
                    }
                    let link = format!("{name}: {}", ty.name);
                    (ty, link)
                }
                search::RustHolds::Call(parts) => {
                    // A local closure named like a function is a value.
                    if !search::bindings(Kind::Rust, text, at + 1, &parts[0]).is_empty() {
                        return None;
                    }
                    // Declared at the top of the file or through a `use`, else an item the
                    // line sees, as a `fn` in the test module around it (#363).
                    let decl = match self.declaration(Kind::Rust, file, &parts) {
                        Some(decl) => decl,
                        None => {
                            let [f] = parts.as_slice() else { return None };
                            let ns = search::RustNamespace::Other;
                            let [l] = search::rust_scope_items(text, at + 1, f, ns)[..] else {
                                return None;
                            };
                            Hit {
                                path: file.to_path_buf(),
                                line: l,
                                col: 0,
                                text: lines[l - 1].to_owned(),
                                deleted: None,
                            }
                        }
                    };
                    if !decl.text.contains("fn ") {
                        return None;
                    }
                    let t = self.text_of(&decl.path)?;
                    let l: Vec<&str> = t.lines().collect();
                    let returns = search::rust_return_type(&l, decl.line - 1)?;
                    let ty = self.rust_resolve(&decl.path, &returns, decl.line - 1)?;
                    let link = format!("{}() -> {}", parts.join("::"), ty.name);
                    (ty, link)
                }
            };
            match &found {
                Some((ty, _)) if (&ty.path, ty.line) != (&this.0.path, this.0.line) => return None,
                Some(_) => {}
                None => found = Some(this),
            }
        }
        found
    }

    /// The `struct`, `enum` or `union` the type `written` on 0-based line `at` of `file` names:
    /// declared once in the file, else the one a `use` of the file binds its name to, else the
    /// project's only one. `Self` is the type of the `impl` around the line. A generic parameter,
    /// a type outside the project and one declared twice with nothing to tell which are none.
    fn rust_resolve(&self, file: &Path, written: &str, at: usize) -> Option<Typed> {
        let text = self.text_of(file)?;
        let (path, mut name) = search::rust_type_name(written)?;
        if name == "Self" && path.is_empty() {
            let lines: Vec<&str> = text.lines().collect();
            let (t, _) = search::rust_self_type(&lines, at)?;
            name = t;
        }
        if search::rust_generic(&text, &name) {
            return None;
        }
        let imports = search::imports(Kind::Rust, &text);
        let ours = |p: &[String]| {
            p.first()
                .is_some_and(|f| matches!(f.as_str(), "crate" | "self" | "super"))
        };
        // `io::Error` behind a `use std::io;` is no project type, whatever the project declares.
        if let Some(first) = path.first()
            && !ours(&path)
            && !bound(&imports, first).is_some_and(|p| ours(&p))
        {
            return None;
        }
        let pattern = search::rust_type_decl_pattern(&name);
        let typed = |hit: Hit| Typed {
            name: name.clone(),
            path: hit.path,
            line: hit.line,
        };
        let decls = self
            .grep(&pattern, false, false, |p| {
                p.extension().is_some_and(|e| e == "rs")
            })
            .unwrap_or_default()
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    search::literal_lines(Kind::Rust, &t).get(h.line - 1) != Some(&true)
                })
            })
            .collect::<Vec<_>>();
        let mine: Vec<&Hit> = decls.iter().filter(|h| h.path == file).collect();
        if let [one] = mine.as_slice() {
            return Some(typed((*one).clone()));
        }
        if !mine.is_empty() {
            return None;
        }
        if bound(&imports, &name).is_some() {
            let hit = self.declaration(Kind::Rust, file, &[name.clone()])?;
            return decls
                .into_iter()
                .find(|h| (&h.path, h.line) == (&hit.path, hit.line))
                .map(typed);
        }
        <[Hit; 1]>::try_from(decls).ok().map(|[hit]| typed(hit))
    }

    /// The field `word` the struct `ty` declares, and its type as written.
    fn rust_field(&self, ty: &Typed, word: &str) -> Option<(Hit, String)> {
        let text = self.text_of(&ty.path)?;
        let lines: Vec<&str> = text.lines().collect();
        let (line, written) = search::rust_struct_field(&lines, ty.line - 1, word)?;
        let hit = Hit {
            path: ty.path.clone(),
            line: line + 1,
            col: 0,
            text: lines[line].to_owned(),
            deleted: None,
        };
        Some((hit, written))
    }

    /// The methods `word` of the type `ty`: in the `impl T` and `impl Tr for T` blocks of its
    /// crate, else the ones with a body in the traits those `impl`s name. A crate that declares
    /// two types of that name keeps to the file of `ty`.
    fn rust_members(&self, ty: &Typed, word: &str) -> Vec<Hit> {
        let crate_of = |p: &Path| {
            p.ancestors()
                .skip(1)
                .find(|d| self.files.contains(&d.join("Cargo.toml")))
                .map(Path::to_path_buf)
        };
        let own = crate_of(&ty.path);
        let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
        let files: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|f| rs(f) && crate_of(f) == own)
            .cloned()
            .collect();
        let namesakes = self
            .grep_in(&search::rust_type_decl_pattern(&ty.name), &files)
            .len();
        let files: Vec<PathBuf> = match namesakes {
            0 | 1 => files,
            _ => vec![ty.path.clone()],
        };
        let method = |hits: Vec<Hit>, owner: &dyn Fn(&search::RustOwner, &str) -> bool| {
            hits.into_iter()
                .filter(|h| {
                    self.text_of(&h.path).is_some_and(|t| {
                        let lines: Vec<&str> = t.lines().collect();
                        search::literal_lines(Kind::Rust, &t).get(h.line - 1) != Some(&true)
                            && search::rust_method_at(&lines, h.line)
                                .is_some_and(|(o, _, at)| owner(&o, lines[at - 1]))
                    })
                })
                .collect::<Vec<Hit>>()
        };
        let pattern = search::rust_method_pattern(word);
        let found = method(self.grep_in(&pattern, &files), &|o, line| {
            matches!(
                o,
                search::RustOwner::Inherent | search::RustOwner::ImplOf(_)
            ) && search::rust_impl_type(line).is_some_and(|t| t == ty.name)
        });
        if !found.is_empty() {
            return found;
        }
        let impls = format!(
            r"^\s*(?:unsafe\s+)?impl\b[^{{]*\bfor\s+&?(?:\w+::)*{}\b",
            regex::escape(&ty.name)
        );
        let traits: Vec<String> = self
            .grep_in(&impls, &files)
            .iter()
            .filter_map(|h| search::rust_impl_trait(&h.text))
            .collect();
        if traits.is_empty() {
            return Vec::new();
        }
        let hits = self
            .grep(&pattern, false, false, |p| rs(p))
            .unwrap_or_default();
        method(
            hits,
            &|o, _| matches!(o, search::RustOwner::Trait(t) if traits.contains(t)),
        )
        .into_iter()
        .filter(|h| !h.text.trim_end().ends_with(';'))
        .collect()
    }
}
