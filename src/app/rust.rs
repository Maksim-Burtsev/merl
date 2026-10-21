mod chain;
mod sysroot;
mod used;

use std::sync::LazyLock;

use super::{
    App, Candidate, HashMap, Hit, Kind, Path, PathBuf, Reason, Regex, Typed, bound, search,
};

pub(super) enum Typing {
    Members(Vec<Candidate>),
    Std(Vec<Candidate>),
}

#[derive(Clone, Copy)]
pub(super) enum MacroUse {
    Macro,
    NotMacro,
    Either,
}

impl MacroUse {
    pub(super) fn after_word(after_word: &str) -> Self {
        let after = after_word.trim_start();
        if after.starts_with('!') && !after.starts_with("!=") {
            Self::Macro
        } else if after.starts_with(['(', ':']) {
            Self::NotMacro
        } else {
            Self::Either
        }
    }
}

fn declares_mod(text: &str, name: &str) -> bool {
    Regex::new(&format!(
        r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+{}\b",
        regex::escape(name)
    ))
    .is_ok_and(|re| re.is_match(text))
}

pub(super) fn tuple_field(name: &str) -> bool {
    !name.starts_with(|c: char| c.is_alphabetic() || c == '_')
}

struct CrateSrc {
    src: PathBuf,
    files: Vec<PathBuf>,
}

struct InCrate {
    module: Vec<String>,
    hits: Vec<Hit>,
    proven: bool,
}

impl App {
    pub(super) fn rust_early(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        range: std::ops::Range<usize>,
        dotted: bool,
    ) -> Option<Vec<Candidate>> {
        let kind = Kind::Rust;
        let after = self.line_str().get(range.end..)?;
        if let Some(attr) =
            search::rust_attribute(&self.buf.lines, self.line, range.start, range.end)
        {
            return Some(match attr {
                search::RustAttr::Nothing => Vec::new(),
                _ => self.rust_macros(attr, word),
            });
        }
        if !dotted && let Some(found) = self.rust_trait_impl(here, text, word, range.clone()) {
            return Some(found);
        }
        if !dotted && let Some(found) = self.rust_self_path(here, text, word, range.start) {
            return Some(found);
        }
        let named = word.starts_with(|c: char| c.is_alphabetic() || c == '_');
        if dotted && named && word != "await" && !after.starts_with('(') && !after.starts_with("::")
        {
            return Some(self.rust_fields(here, word));
        }
        let before = self.line_str().get(..range.start)?.trim();
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
                    line1: self.line + 1,
                    byte_col: None,
                    deleted: None,
                    text: self.line_str().to_owned(),
                },
                reason: Reason::ByName,
            });
            return Some(found);
        }
        None
    }

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
                    .is_some_and(|t| search::rust_proc_macro(&t, h.line1))
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    fn rust_fields(&mut self, here: &Path, word: &str) -> Vec<Candidate> {
        let kind = Kind::Rust;
        let field = |this: &Self, h: &Hit| {
            this.text_of(&h.path)
                .is_some_and(|t| search::rust_field_at(&t, h.line1))
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
                .is_some_and(|t| search::rust_variant_at(&t, h.line1))
        });
        hits
    }

    pub(super) fn rust_glob_variant(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        imports: &[search::Import],
    ) -> Vec<Candidate> {
        let kind = Kind::Rust;
        let search::RustGlobUses {
            paths_before_star: globs,
            own_to_function: own,
        } = search::rust_glob_uses(text, self.line + 1);
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
                let of = search::qualified(kind, &t, v.line1, word)
                    .is_some_and(|q| q == full || q.ends_with(&format!("::{full}")));
                let in_enum = of
                    && enums.iter().any(|e| {
                        e.path == v.path && (!mine || e.path == here) && e.line1 < v.line1
                    });
                if in_enum
                    && !found
                        .iter()
                        .any(|c: &Candidate| (&c.hit.path, c.hit.line1) == (&v.path, v.line1))
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

    pub(super) fn rust_file_items(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        range: std::ops::Range<usize>,
    ) -> Vec<Candidate> {
        let Some(after) = self.line_str().get(range.end..).map(str::trim_start) else {
            return Vec::new();
        };
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
        let line = self.line_str();
        let right_of_own_let = !search::rust_let_declares(line, range.start, word)
            && line
                .match_indices(word)
                .any(|(i, _)| i < range.start && search::rust_let_declares(line, i, word));
        let items = if right_of_own_let {
            search::rust_scope_items_right_of_let(text, self.line + 1, word, ns)
        } else {
            search::rust_scope_items(text, self.line + 1, word, ns)
        };
        items
            .into_iter()
            .map(|line| Candidate {
                hit: Hit {
                    deleted: None,
                    path: here.to_path_buf(),
                    line1: line,
                    byte_col: None,
                    text: self.buf.lines[line - 1].clone(),
                },
                reason: Reason::File,
            })
            .collect()
    }

    #[expect(
        clippy::match_same_arms,
        reason = "the visibility table reads row by row, in order of precedence"
    )]
    pub(super) fn rust_methods(
        &mut self,
        here: &Path,
        word: &str,
        text: &str,
        chain: &[String],
        before: &str,
        std: bool,
    ) -> Vec<Candidate> {
        let kind = Kind::Rust;
        let receiver = rust_receiver_type(text, self.line, chain, before);
        let pattern = search::rust_method_pattern(word);
        let mut hits = self.project_definitions(kind, here, word, &pattern);
        let files = self.external_files(kind);
        hits.extend(self.external_grep(kind, &files, &pattern));
        hits.retain(|h| h.path != here || h.line1 != self.line + 1);
        let package = |dir: &Path| self.rust_package(dir);
        let own = self.rust_crate_of(here);
        let lock = std::fs::read_to_string(self.root.join("Cargo.lock")).unwrap_or_default();
        let reach = own
            .as_deref()
            .and_then(package)
            .and_then(|name| search::cargo_reach(&lock, &name));
        let workspace: Vec<String> = self
            .files
            .iter()
            .filter(|f| f.file_name().is_some_and(|n| n == "Cargo.toml"))
            .filter_map(|f| f.parent().and_then(package))
            .collect();
        let roots = self
            .external
            .get(&kind)
            .map(|(roots, _)| roots.clone())
            .unwrap_or_default();
        let mut texts: HashMap<PathBuf, String> = HashMap::new();
        let mut rows: Vec<(Hit, search::RustOwner)> = Vec::new();
        let mut hidden: Vec<String> = Vec::new();
        let mut receiver_impls: Vec<String> = Vec::new();
        let mut own_impls: Vec<(Hit, String)> = Vec::new();
        for h in hits {
            if !texts.contains_key(&h.path) {
                let text = self.text_of(&h.path).unwrap_or_default();
                texts.insert(h.path.clone(), text);
            }
            let lines: Vec<&str> = texts[&h.path].lines().collect();
            let Some(search::RustMethod {
                owner,
                vis,
                owner_line1: owner_line,
            }) = search::rust_method_at(&lines, h.line1)
            else {
                continue;
            };
            let inside = !h.path.is_absolute();
            let rel = roots
                .iter()
                .find_map(|r| h.path.strip_prefix(r).ok())
                .unwrap_or(&h.path);
            let sysroot = h
                .path
                .to_string_lossy()
                .contains("rustlib/src/rust/library/");
            let unmarked = sysroot
                && match owner {
                    search::RustOwner::Inherent => !search::rust_stability(&lines, h.line1),
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
            let visible = !(internal || (std && inside && owner == search::RustOwner::Inherent))
                && match (&owner, vis) {
                    (search::RustOwner::ImplOf(_), _) => true,
                    (search::RustOwner::Trait(_), search::RustVis::Pub) => true,
                    (search::RustOwner::Trait(_), _) => inside,
                    (_, search::RustVis::Pub) => true,
                    (_, search::RustVis::Crate) => inside && self.rust_crate_of(&h.path) == own,
                    (search::RustOwner::Inherent, search::RustVis::Private) => {
                        inside && rust_below(here, &h.path)
                    }
                    (_, search::RustVis::Private) => true,
                };
            if let (search::RustOwner::Trait(t), false) = (&owner, visible) {
                hidden.push(t.clone());
            }
            if visible && self.rust_reached(&h, reach.as_ref(), &workspace) {
                if let search::RustOwner::ImplOf(t) = &owner
                    && receiver.is_some()
                    && search::rust_impl_type(lines[owner_line - 1]) == receiver
                {
                    if !inside {
                        receiver_impls.push(t.clone());
                    } else if receiver.as_deref() == Some("str") {
                        own_impls.push((h.clone(), t.clone()));
                    }
                }
                rows.push((h, owner));
            }
        }
        rust_ranked(rows, &hidden, &receiver_impls, &own_impls)
    }

    fn rust_bound(
        &self,
        text: &str,
        first: &str,
        chain: &[String],
    ) -> Option<Vec<(Vec<String>, usize)>> {
        let lines: Vec<&str> = text.lines().collect();
        let indent = |l: &str| l.len() - l.trim_start().len();
        let use_reaches_cursor = |at: usize| {
            let from = text.bytes().take(at).filter(|&b| b == b'\n').count();
            let depth = indent(lines[from]);
            depth == 0
                || (from < self.line
                    && lines[from + 1..=self.line]
                        .iter()
                        .all(|l| l.trim().is_empty() || indent(l) >= depth))
        };
        let depth_at =
            |at: usize| indent(lines[text.bytes().take(at).filter(|&b| b == b'\n').count()]);
        let mut bound: Vec<(Vec<String>, usize)> = search::rust_uses(text)
            .into_iter()
            .filter(|u| u.name == first && use_reaches_cursor(u.start_byte))
            .map(|u| (u.path, depth_at(u.start_byte)))
            .collect();
        bound.dedup();
        let glob = Regex::new(r"(?m)^[ \t]+use\s[^;]*\*\s*;").expect("a valid pattern");
        if let [(_, depth)] = bound.as_slice()
            && chain.is_empty()
            && glob
                .find_iter(text)
                .any(|m| use_reaches_cursor(m.start()) && depth_at(m.start()) > *depth)
        {
            return None;
        }
        Some(bound)
    }

    fn rust_crate_of(&self, p: &Path) -> Option<PathBuf> {
        p.ancestors()
            .skip(1)
            .find(|d| self.files.contains(&d.join("Cargo.toml")))
            .map(Path::to_path_buf)
    }

    fn rust_package(&self, dir: &Path) -> Option<String> {
        self.text_of(&dir.join("Cargo.toml"))
            .and_then(|t| search::cargo_package_name(&t))
    }

    fn rust_reached(
        &self,
        h: &Hit,
        reach: Option<&search::CargoReach>,
        workspace: &[String],
    ) -> bool {
        let name = if h.path.is_absolute() {
            let listed = h.path.ancestors().find_map(|a| {
                let dir = a.file_name()?.to_str()?;
                let locked = reach.map(|r| r.every_locked.as_slice()).unwrap_or_default();
                locked
                    .iter()
                    .find(|p| p.registry_dir == dir)
                    .map(|p| p.name.clone())
            });
            if listed.as_ref().is_some_and(|n| workspace.contains(n)) {
                return false;
            }
            listed
        } else {
            self.rust_crate_of(&h.path)
                .as_deref()
                .and_then(|d| self.rust_package(d))
        };
        let Some(search::CargoReach {
            reached,
            every_locked,
        }) = reach
        else {
            return true;
        };
        name.is_none_or(|n| reached.contains(&n) || !every_locked.iter().any(|p| p.name == n))
    }

    pub(super) fn rust_crate_path(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        chain: &[String],
        macro_call: MacroUse,
    ) -> Option<Vec<Candidate>> {
        const PRIMITIVES: &[&str] = &[
            "bool", "char", "str", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32",
            "i64", "i128", "isize", "f32", "f64",
        ];
        const SYSROOT: &[&str] = &["std", "core", "alloc", "proc_macro"];
        let first = chain.first().map_or(word, String::as_str);
        let bound = self.rust_bound(text, first, chain)?;
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
        let own_mod = declares_mod(text, root);
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
            let files = self.sysroot_crate("core")?.files;
            let name = rest.last()?;
            let pattern = search::def_patterns(kind, name).join("|");
            let hits = search::grep_project(&self.root, &files, &pattern, false, false, None, None)
                .unwrap_or_default();
            self.note_cut(&hits);
            let hits = self.declaring(kind, name, hits);
            return (!hits.is_empty()).then(|| by(hits, Reason::ByName));
        }
        let (krate, start_module, name) = match root.as_str() {
            "crate" | "self" | "super" => {
                let (krate, module) = self.rust_own_crate(here, root, rest)?;
                (krate, module, "crate".to_owned())
            }
            _ if own_mod => return None,
            r if SYSROOT.contains(&r) => (self.sysroot_crate(r)?, Vec::new(), root.clone()),
            r => (
                self.workspace_crate(r).or_else(|| self.registry_crate(r))?,
                Vec::new(),
                root.clone(),
            ),
        };
        let rest: Vec<String> = rest.iter().filter(|p| *p != "super").cloned().collect();
        let sep = "::";
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
        let found = self.rust_in_crate(&krate, &start_module, &rest, macro_call, 0);
        if let Some(InCrate {
            module,
            hits,
            proven,
        }) = found
        {
            return Some(by(hits, reason(&name, module, proven)));
        }
        if root == "std" {
            for other in ["core", "alloc"] {
                let Some(krate) = self.sysroot_crate(other) else {
                    continue;
                };
                if let Some(InCrate {
                    module,
                    hits,
                    proven,
                }) = self.rust_in_crate(&krate, &[], &rest, macro_call, 0)
                {
                    return Some(by(hits, reason(other, module, proven)));
                }
            }
        }
        None
    }

    fn rust_own_crate(
        &self,
        here: &Path,
        root: &str,
        rest: &[String],
    ) -> Option<(CrateSrc, Vec<String>)> {
        let search::RustModule {
            crate_src: src,
            path: mut module,
        } = search::rust_module_of(&self.files, here)?;
        match root {
            "crate" => module.clear(),
            "super" => {
                let supers = 1 + rest.iter().take_while(|p| *p == "super").count();
                module.truncate(module.len().checked_sub(supers)?);
            }
            "self" => {}
            _ => return None,
        }
        let files: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|f| f.starts_with(&src))
            .cloned()
            .collect();
        Some((CrateSrc { src, files }, module))
    }

    fn rust_in_crate(
        &self,
        krate: &CrateSrc,
        base: &[String],
        path: &[String],
        macro_call: MacroUse,
        depth: usize,
    ) -> Option<InCrate> {
        let CrateSrc { src, files } = krate;
        let (name, parts) = path.split_last()?;
        let mut module: Vec<String> = base.to_vec();
        module.extend(parts.iter().cloned());
        let file_of = |m: &[String]| -> Vec<PathBuf> {
            let at = m.iter().fold(src.clone(), |p, n| p.join(n));
            let wanted = if m.is_empty() {
                vec![src.join("lib.rs"), src.join("main.rs")]
            } else {
                vec![at.with_extension("rs"), at.join("mod.rs")]
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
            return Some(InCrate {
                module: at.to_vec(),
                hits,
                proven: true,
            });
        }
        let first = inside.first().unwrap_or(name);
        let taken = here.iter().find_map(|f| {
            let text = self.text_of(f)?;
            let mut bound = search::rust_uses(&text)
                .into_iter()
                .filter(|u| u.name == *first && u.in_column_zero)
                .map(|u| u.path);
            let p = bound.next()?;
            let to: Vec<String> = match p.first()?.as_str() {
                "crate" => p[1..].to_vec(),
                "super" => {
                    let up = p.iter().take_while(|s| *s == "super").count();
                    let kept = at.len().checked_sub(up)?;
                    at[..kept].iter().chain(&p[up..]).cloned().collect()
                }
                _ => at.iter().chain(&p).cloned().collect(),
            };
            Some(to)
        });
        if let Some(mut to) = taken.filter(|_| depth < 4) {
            to.extend(inside.iter().skip(1).cloned());
            if !inside.is_empty() {
                to.push(name.clone());
            }
            if let Some(found) = self.rust_in_crate(krate, &[], &to, macro_call, depth + 1) {
                return Some(found);
            }
        }
        let dir = at.iter().fold(src.clone(), |p, m| p.join(m));
        let under: Vec<PathBuf> = files
            .iter()
            .filter(|f| f.starts_with(&dir))
            .cloned()
            .collect();
        let hits = self.rust_declared(&under, name, owner.as_deref(), macro_call);
        if !hits.is_empty() {
            return Some(InCrate {
                module: at.to_vec(),
                hits,
                proven: true,
            });
        }
        let hits = match owner {
            Some(_) => self.rust_declared(files, name, owner.as_deref(), macro_call),
            None => Vec::new(),
        };
        (!hits.is_empty()).then(|| InCrate {
            module: at.to_vec(),
            hits,
            proven: false,
        })
    }

    fn rust_declared(
        &self,
        files: &[PathBuf],
        name: &str,
        owner: Option<&str>,
        macro_call: MacroUse,
    ) -> Vec<Hit> {
        let kind = Kind::Rust;
        let pattern = search::def_patterns(kind, name).join("|");
        let hits = search::grep_project(&self.root, files, &pattern, false, false, None, None)
            .unwrap_or_default();
        self.note_cut(&hits);
        let want = owner.map(|o| format!("{o}::{name}"));
        let mut hits = self.declaring(kind, name, hits);
        hits.retain(|h| {
            let macro_rules = h.text.trim_start().starts_with("macro_rules!");
            let namespace_fits = match macro_call {
                MacroUse::Macro => macro_rules,
                MacroUse::NotMacro => !macro_rules,
                MacroUse::Either => true,
            };
            namespace_fits
                && self
                    .text_of(&h.path)
                    .map(|t| search::qualified(kind, &t, h.line1, name))
                    == Some(want.clone())
        });
        hits
    }

    fn sysroot_crate(&mut self, name: &str) -> Option<CrateSrc> {
        let all = self.external_files(Kind::Rust);
        let roots = self.external.get(&Kind::Rust)?.0.clone();
        roots.iter().find_map(|r| {
            let src = r.join(name).join("src");
            let files: Vec<PathBuf> = all
                .iter()
                .filter(|f| f.starts_with(&src))
                .cloned()
                .collect();
            (!files.is_empty()).then_some(CrateSrc { src, files })
        })
    }

    fn registry_crate(&mut self, name: &str) -> Option<CrateSrc> {
        let all = self.external_files(Kind::Rust);
        let roots = self.external.get(&Kind::Rust)?.0.clone();
        roots.iter().find_map(|r| {
            let dir = r.file_name()?.to_string_lossy().into_owned();
            let (i, _) = dir.rmatch_indices('-').find(|(i, _)| {
                dir.get(i + 1..)
                    .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()))
            })?;
            if dir.get(..i)?.replace('-', "_") != name {
                return None;
            }
            let src = r.join("src");
            let files: Vec<PathBuf> = all
                .iter()
                .filter(|f| f.starts_with(&src))
                .cloned()
                .collect();
            (!files.is_empty()).then_some(CrateSrc { src, files })
        })
    }

    fn workspace_crate(&self, name: &str) -> Option<CrateSrc> {
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
                (named && !files.is_empty()).then_some(CrateSrc { src, files })
            })
    }

    pub(super) fn rust_typed(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        chain: &[String],
        call: bool,
    ) -> Result<Typing, String> {
        if let Some(seventh) = chain.get(6) {
            return Err(seventh.clone());
        }
        let Some((mut ty, link)) = self.rust_value_type(here, text, self.line, &chain[0]) else {
            if let [name] = chain
                && call
            {
                if let Some(found) = self.rust_trait_bound(here, text, word, name) {
                    return Ok(Typing::Members(found));
                }
                if let Some(written) = self.rust_std_written(here, text, name)
                    && let Some(found) = self.rust_std_call(text, name, &written, word)
                {
                    return Ok(Typing::Std(found));
                }
            }
            return Err(chain[0].clone());
        };
        let mut links = vec![link];
        for (i, field) in chain.iter().enumerate().skip(1) {
            let (hit, written) = self.rust_field(&ty, field).ok_or_else(|| field.clone())?;
            let Some(next) = self.rust_resolve(&ty.path, &written, hit.line1 - 1) else {
                if call
                    && i + 1 == chain.len()
                    && let Some(std) = rust_bare_type(&written)
                    && let Some(owner) = self.text_of(&ty.path)
                    && let Some(found) = self.rust_std_call(&owner, field, &std, word)
                {
                    return Ok(Typing::Std(found));
                }
                return Err(field.clone());
            };
            match i {
                1 => links[0] = format!("{}.{field}: {}", chain[0], next.name),
                _ => links.push(format!("{field}: {}", next.name)),
            }
            ty = next;
        }
        let hits = if call {
            self.rust_members(&ty, word)
        } else {
            self.rust_field(&ty, word)
                .map(|(hit, _)| vec![hit])
                .unwrap_or_default()
        };
        let label = links.join(" \u{2192} ");
        Ok(Typing::Members(
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect(),
        ))
    }

    fn rust_value_type(
        &self,
        file: &Path,
        text: &str,
        line0: usize,
        name: &str,
    ) -> Option<(Typed, String)> {
        let lines: Vec<&str> = text.lines().collect();
        if name == "self" {
            let search::RustSelfType {
                written,
                impl_line0: impl_line,
            } = search::rust_self_type(&lines, line0)?;
            let ty = self.rust_resolve(file, &written, impl_line)?;
            let link = format!("self: {}", ty.name);
            return Some((ty, link));
        }
        let bindings = search::bindings(Kind::Rust, text, line0 + 1, name);
        let mut found: Option<(Typed, String)> = None;
        for b in &bindings {
            let at = b.line1 - 1;
            let Some(holds) = search::rust_holds(&lines, at, name) else {
                let ty = self.rust_let_chain(file, text, at, name)?;
                let this = (ty.clone(), format!("{name}: {}", ty.name));
                match &found {
                    Some((t, _)) if (&t.path, t.line) != (&ty.path, ty.line) => return None,
                    Some(_) => {}
                    None => found = Some(this),
                }
                continue;
            };
            let this = match holds {
                search::RustHolds::Type(w) | search::RustHolds::Literal(w) => {
                    let ty = self.rust_resolve(file, &w, at)?;
                    let link = format!("{name}: {}", ty.name);
                    (ty, link)
                }
                search::RustHolds::AssocCall {
                    written_type: w,
                    function: f,
                } => {
                    let ty = self.rust_resolve(file, &w, at)?;
                    let returns = match <[Hit; 1]>::try_from(self.rust_members(&ty, &f)) {
                        Ok([decl]) => {
                            let t = self.text_of(&decl.path)?;
                            let l: Vec<&str> = t.lines().collect();
                            let n = search::rust_type_name(&search::rust_return_type(
                                &l,
                                decl.line1 - 1,
                            )?)?
                            .name;
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
                    if !search::bindings(Kind::Rust, text, at + 1, &parts[0]).is_empty() {
                        return None;
                    }
                    let decl = if let Some(decl) = self.declaration(Kind::Rust, file, &parts) {
                        decl
                    } else {
                        let [f] = parts.as_slice() else { return None };
                        let ns = search::RustNamespace::Other;
                        let [l] = search::rust_scope_items_right_of_let(text, at + 1, f, ns)[..]
                        else {
                            return None;
                        };
                        Hit {
                            path: file.to_path_buf(),
                            line1: l,
                            byte_col: None,
                            text: lines[l - 1].to_owned(),
                            deleted: None,
                        }
                    };
                    if !decl.text.contains("fn ") {
                        return None;
                    }
                    let t = self.text_of(&decl.path)?;
                    let l: Vec<&str> = t.lines().collect();
                    let returns = search::rust_return_type(&l, decl.line1 - 1)?;
                    let ty = self.rust_resolve(&decl.path, &returns, decl.line1 - 1)?;
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

    fn rust_resolve(&self, file: &Path, written: &str, line0: usize) -> Option<Typed> {
        let text = self.text_of(file)?;
        let lines: Vec<&str> = text.lines().collect();
        let search::RustTypeName { path, mut name } = search::rust_type_name(written)?;
        if name == "Self" && path.is_empty() {
            name = search::rust_self_type(&lines, line0)?.written;
        }
        if search::rust_generic(&text, &name) {
            return None;
        }
        if path.is_empty() {
            let literal = search::literal_lines(Kind::Rust, &text);
            let used = search::rust_inline_mod_past(&lines, &literal, line0)
                .and_then(|m| self.rust_used_type(file, &text, &name, Some(m), line0))
                .or_else(|| {
                    search::rust_inline_mod(&lines, line0)
                        .is_none()
                        .then(|| self.rust_used_type(file, &text, &name, None, line0))?
                });
            if used.is_some() {
                return used;
            }
        }
        let imports = search::imports(Kind::Rust, &text);
        let ours = |p: &[String]| {
            p.first()
                .is_some_and(|f| matches!(f.as_str(), "crate" | "self" | "super"))
        };
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
            line: hit.line1,
        };
        let decls = self
            .grep(&pattern, false, false, |p| {
                p.extension().is_some_and(|e| e == "rs")
            })
            .unwrap_or_default()
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) != Some(&true)
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
                .find(|h| (&h.path, h.line1) == (&hit.path, hit.line1))
                .map(typed);
        }
        if decls.is_empty() {
            return self.rust_alias(&name);
        }
        <[Hit; 1]>::try_from(decls).ok().map(|[hit]| typed(hit))
    }

    fn rust_field(&self, ty: &Typed, word: &str) -> Option<(Hit, String)> {
        let text = self.text_of(&ty.path)?;
        let lines: Vec<&str> = text.lines().collect();
        let search::RustStructField {
            line0: line,
            written_type: written,
        } = search::rust_struct_field(&lines, ty.line - 1, word)?;
        let hit = Hit {
            path: ty.path.clone(),
            line1: line + 1,
            byte_col: None,
            text: lines[line].to_owned(),
            deleted: None,
        };
        Some((hit, written))
    }

    fn rust_crate_rs(&self, p: &Path) -> Vec<PathBuf> {
        let own = self.rust_crate_of(p);
        self.files
            .iter()
            .filter(|f| f.extension().is_some_and(|e| e == "rs") && self.rust_crate_of(f) == own)
            .cloned()
            .collect()
    }

    fn rust_type_files(&self, ty: &Typed) -> Vec<PathBuf> {
        let files = self.rust_crate_rs(&ty.path);
        let decl = search::rust_type_decl_pattern(&ty.name);
        match self.grep_in(&decl, &files).len() {
            0 | 1 => files,
            _ => vec![ty.path.clone()],
        }
    }

    fn rust_trait_impl(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        range: std::ops::Range<usize>,
    ) -> Option<Vec<Candidate>> {
        static AS: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"<\s*([\w:]+)\s+as\s+(?:\w+::)*([A-Za-z_]\w*)\s*>::$")
                .expect("the regex is a literal")
        });
        static PATH: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?:^|[^\w:>])(?:\w+::)*([A-Z]\w*)::$").expect("the regex is a literal")
        });
        static ARG: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\(\s*&?(?:mut\s+)?([a-z_]\w*)\s*[,)]").expect("the regex is a literal")
        });
        static TAKES_SELF: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"\bfn\s+\w+[^(]*\(\s*(?:&\s*(?:'\w+\s+)?)?(?:mut\s+)?self\b")
                .expect("the regex is a literal")
        });
        let line = self.line_str();
        let before = line.get(..range.start)?;
        let (ty, tr, by_arg) = if let Some(c) = AS.captures(before) {
            (
                self.rust_resolve(here, &c[1], self.line)?,
                c[2].to_owned(),
                false,
            )
        } else {
            let tr = PATH.captures(before)?[1].to_owned();
            let arg = ARG.captures(line.get(range.end..)?)?;
            (
                self.rust_value_type(here, text, self.line, &arg[1])?.0,
                tr,
                true,
            )
        };
        let files = self.rust_type_files(&ty);
        let hits: Vec<Hit> = self
            .grep_in(&search::rust_method_pattern(word), &files)
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    let lines: Vec<&str> = t.lines().collect();
                    search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) != Some(&true)
                        && (!by_arg
                            || TAKES_SELF.is_match(
                                &lines
                                    .iter()
                                    .skip(h.line1 - 1)
                                    .take(3)
                                    .copied()
                                    .collect::<Vec<_>>()
                                    .join(" "),
                            ))
                        && search::rust_method_at(&lines, h.line1).is_some_and(|m| {
                            m.owner == search::RustOwner::ImplOf(tr.clone())
                                && search::rust_impl_type(lines[m.owner_line1 - 1])
                                    .is_some_and(|t| t == ty.name)
                        })
                })
            })
            .collect();
        let [hit] = <[Hit; 1]>::try_from(hits).ok()?;
        Some(vec![Candidate {
            hit,
            reason: Reason::Receiver(format!("<{} as {tr}>", ty.name)),
        }])
    }

    fn rust_self_path(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        start: usize,
    ) -> Option<Vec<Candidate>> {
        let before = self.line_str().get(..start)?;
        let pathed = before
            .strip_suffix("Self::")
            .is_some_and(|b| !b.ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == ':'));
        if word != "Self" && !pathed {
            return None;
        }
        let lines: Vec<&str> = text.lines().collect();
        let search::RustSelfType {
            written,
            impl_line0,
        } = search::rust_self_type(&lines, self.line)?;
        if word == "Self" {
            return Some(vec![Candidate {
                hit: Hit {
                    deleted: None,
                    path: here.to_path_buf(),
                    line1: impl_line0 + 1,
                    byte_col: None,
                    text: lines[impl_line0].to_owned(),
                },
                reason: Reason::File,
            }]);
        }
        let ty = self.rust_resolve(here, &written, impl_line0)?;
        let found = self.rust_members(&ty, word);
        (!found.is_empty()).then(|| {
            found
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(format!("Self: {}", ty.name)),
                })
                .collect()
        })
    }

    fn rust_trait_bound(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        name: &str,
    ) -> Option<Vec<Candidate>> {
        let lines: Vec<&str> = text.lines().collect();
        let (files, own, traits) = if name == "self" {
            let t = search::rust_self_trait(&lines, self.line)?;
            (vec![here.to_path_buf()], Some(t + 1), Vec::new())
        } else {
            let [b] = &search::bindings(Kind::Rust, text, self.line + 1, name)[..] else {
                return None;
            };
            let search::RustHolds::Type(w) = search::rust_holds(&lines, b.line1 - 1, name)? else {
                return None;
            };
            let param = w.trim_start_matches('&').trim_start_matches("mut ").trim();
            let traits = search::rust_bounds(&lines, b.line1 - 1, param);
            if traits.is_empty() {
                return None;
            }
            (self.rust_crate_rs(here), None, traits)
        };
        let found: Vec<(Hit, String)> = self
            .grep_in(&search::rust_method_pattern(word), &files)
            .into_iter()
            .filter_map(|h| {
                let t = self.text_of(&h.path)?;
                let lines: Vec<&str> = t.lines().collect();
                if search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) == Some(&true) {
                    return None;
                }
                let m = search::rust_method_at(&lines, h.line1)?;
                let search::RustOwner::Trait(tr) = m.owner else {
                    return None;
                };
                let fits = match own {
                    Some(line1) => m.owner_line1 == line1,
                    None => traits.contains(&tr),
                };
                fits.then_some((h, tr))
            })
            .collect();
        let [(hit, tr)] = <[(Hit, String); 1]>::try_from(found).ok()?;
        Some(vec![Candidate {
            hit,
            reason: Reason::Trait(tr),
        }])
    }

    pub(super) fn rust_macro_declared(&self, word: &str) -> Vec<Candidate> {
        static MATCHER: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s*[(\[{]\s*\$(\w+)\s*:\s*ident\b").expect("the regex is a literal")
        });
        if !word.starts_with(|c: char| c.is_alphabetic() || c == '_') {
            return Vec::new();
        }
        let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
        let call = format!(r"^\s*(\w+)!\s*[(\[{{]\s*{}\b", regex::escape(word));
        let Ok(call_re) = Regex::new(&call) else {
            return Vec::new();
        };
        let declares = |m: &str| {
            let rules = format!(
                r"^\s*(?:#\[[^\]]*\]\s*)*macro_rules!\s*{}\b",
                regex::escape(m)
            );
            let [h] = &self.grep(&rules, false, false, rs).unwrap_or_default()[..] else {
                return false;
            };
            let Some(t) = self.text_of(&h.path) else {
                return false;
            };
            let lines: Vec<&str> = t.lines().collect();
            let depth = lines[h.line1 - 1].len() - lines[h.line1 - 1].trim_start().len();
            let body: Vec<&str> = lines[h.line1..]
                .iter()
                .take_while(|l| l.trim().is_empty() || l.len() - l.trim_start().len() > depth)
                .copied()
                .collect();
            let Some(x) = body
                .iter()
                .find(|l| !l.trim().is_empty())
                .and_then(|l| MATCHER.captures(l))
            else {
                return false;
            };
            Regex::new(&format!(
                r"\b(?:struct|enum|union|fn|trait|type|mod|const|static)\s+\${}\b",
                &x[1]
            ))
            .is_ok_and(|re| body.iter().any(|l| re.is_match(l)))
        };
        self.grep(&call, false, false, rs)
            .unwrap_or_default()
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) != Some(&true)
                }) && call_re
                    .captures(&h.text)
                    .is_some_and(|c| c[1] != *"macro_rules" && declares(&c[1]))
            })
            .map(|hit| Candidate {
                reason: Reason::ByName,
                hit,
            })
            .collect()
    }

    pub(super) fn rust_alias_members(&self, alias: &str, word: &str) -> Vec<Candidate> {
        let Some(ty) = self.rust_alias(alias) else {
            return Vec::new();
        };
        let reason = Reason::Path(format!("{alias} = {}", ty.name));
        self.rust_members(&ty, word)
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: reason.clone(),
            })
            .collect()
    }

    fn rust_members(&self, ty: &Typed, word: &str) -> Vec<Hit> {
        let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
        let decl = search::rust_type_decl_pattern(&ty.name);
        let files = self.rust_type_files(ty);
        let method = |hits: Vec<Hit>, owner: &dyn Fn(&search::RustOwner, &str) -> bool| {
            hits.into_iter()
                .filter(|h| {
                    self.text_of(&h.path).is_some_and(|t| {
                        let lines: Vec<&str> = t.lines().collect();
                        search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) != Some(&true)
                            && search::rust_method_at(&lines, h.line1)
                                .is_some_and(|m| owner(&m.owner, lines[m.owner_line1 - 1]))
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
        let text_of_ty = self.text_of(&ty.path).unwrap_or_default();
        let lines_of_ty: Vec<&str> = text_of_ty.lines().collect();
        let home = search::rust_inline_mod(&lines_of_ty, ty.line - 1);
        let declared_twice_here = self.grep_in(&decl, std::slice::from_ref(&ty.path)).len() > 1;
        let found: Vec<Hit> = found
            .into_iter()
            .filter(|h| {
                !declared_twice_here
                    || h.path != ty.path
                    || search::rust_inline_mod(&lines_of_ty, h.line1 - 1) == home
            })
            .collect();
        let inherent: Vec<Hit> = found
            .iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    let lines: Vec<&str> = t.lines().collect();
                    search::rust_method_at(&lines, h.line1)
                        .is_some_and(|m| matches!(m.owner, search::RustOwner::Inherent))
                })
            })
            .cloned()
            .collect();
        if !inherent.is_empty() {
            return inherent;
        }
        if !found.is_empty() {
            return found;
        }
        let impls = format!(
            r"^\s*(?:(?:unsafe|const)\s+)*impl\b[^{{]*\bfor\s+&?(?:\w+::)*{}\b",
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

fn rust_below(here: &Path, p: &Path) -> bool {
    let parent = p.parent().unwrap_or_else(|| Path::new(""));
    let root_file = p
        .file_stem()
        .is_some_and(|s| s == "lib" || s == "main" || s == "mod" || s == "build")
        || ["tests", "benches", "examples"]
            .iter()
            .any(|d| parent.file_name().is_some_and(|n| n == *d))
        || parent.ends_with("src/bin");
    let dir = if root_file {
        parent.to_path_buf()
    } else {
        p.with_extension("")
    };
    here == p || here.starts_with(dir)
}

fn rust_receiver_type(text: &str, line0: usize, chain: &[String], before: &str) -> Option<String> {
    static STR_LITERAL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?:^|[^\w"])"(?:[^"\\]|\\.)*"\.$"#).expect("the regex is a literal")
    });
    match chain {
        [name] => rust_written_type(text, line0, name),
        [] if STR_LITERAL.is_match(before) => Some("str".to_owned()),
        _ => None,
    }
}

fn rust_written_type(text: &str, line0: usize, name: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let bindings = search::bindings(Kind::Rust, text, line0 + 1, name);
    let [b] = bindings.as_slice() else {
        return None;
    };
    let str_literal = Regex::new(&format!(
        r#"^\s*let\s+(?:mut\s+)?{}\s*=\s*"(?:[^"\\]|\\.)*"\s*;\s*$"#,
        regex::escape(name)
    ))
    .ok()?;
    if str_literal.is_match(lines.get(b.line1 - 1)?) {
        return Some("str".to_owned());
    }
    match search::rust_holds(&lines, b.line1 - 1, name)? {
        search::RustHolds::Type(w) | search::RustHolds::Literal(w) => rust_bare_type(&w),
        _ => None,
    }
}

fn rust_bare_type(written: &str) -> Option<String> {
    static BARE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:&\s*(?:'\w+\s+)?(?:mut\s+)?)*(?:\w+::)*([A-Za-z_]\w*)")
            .expect("the regex is a literal")
    });
    BARE.captures(written.trim()).map(|c| c[1].to_owned())
}

fn rust_ranked(
    mut rows: Vec<(Hit, search::RustOwner)>,
    hidden: &[String],
    receiver_impls: &[String],
    own_impls: &[(Hit, String)],
) -> Vec<Candidate> {
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
        && !receiver_impls.contains(tr)
        && rows.iter().all(|(_, o)| {
            matches!(o, search::RustOwner::Trait(t) | search::RustOwner::ImplOf(t) if t == *tr)
        })
    {
        let reason = Reason::Trait((*tr).clone());
        let hit = match own_impls {
            [(hit, t)] if t == *tr => hit.clone(),
            _ => rows.swap_remove(0).0,
        };
        return vec![Candidate { hit, reason }];
    }
    rows.into_iter()
        .map(|(hit, _)| Candidate {
            hit,
            reason: Reason::ByName,
        })
        .collect()
}
