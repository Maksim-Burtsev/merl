use super::{
    App, Asked, Candidate, Hit, Kind, Near, PathBuf, Patterns, Reason, Regex, bound, search,
};

pub(super) struct OwnerPath {
    sep: &'static str,
    pub(super) path: String,
    answer_files: Option<Vec<PathBuf>>,
    mine: Option<String>,
    pub(super) pathed: bool,
    colons: bool,
    class_method: bool,
}

impl App {
    pub(super) fn pathed(
        &mut self,
        a: &mut Asked,
        near: &Near,
        p: &mut Patterns,
    ) -> Option<OwnerPath> {
        let mut q = self.path_of(a, near, p);
        let pathed = q.pathed
            && !a.chain.is_empty()
            && (q.sep == "." || q.mine.is_some() || self.owner_declared_once(a, near, &mut q));
        q.pathed = pathed;
        q.class_method = a.kind == Kind::Ruby
            && a.dotted
            && a.chain.last().is_some_and(|c| {
                c.starts_with(|c: char| c.is_ascii_uppercase())
                    && c.contains(|c: char| c.is_ascii_lowercase())
            });
        if q.class_method && a.word == "new" {
            "initialize".clone_into(&mut a.word);
            p.pattern = search::def_patterns(a.kind, "initialize").join("|");
        }
        if q.pathed
            && near.locals.is_empty()
            && !a.chain.is_empty()
            && self.answered_by_path(a, p, &q)
        {
            return None;
        }
        Some(q)
    }

    fn path_of(&self, a: &Asked, near: &Near, p: &Patterns) -> OwnerPath {
        let (kind, chain) = (a.kind, &a.chain);
        let sep = search::separator(kind);
        let path = chain.join(sep);
        let declared = !a.tail.starts_with("::")
            && Regex::new(r"^\s*(?:class|module)\s+[\w:]*$").is_ok_and(|re| re.is_match(&a.before));
        let colons = kind == Kind::Ruby && a.before.ends_with("::") && !declared;
        let pathed = match sep {
            "." => p.on_value || colons,
            _ => a.lead.ends_with(&format!("{path}{sep}")),
        };
        let seen = if kind == Kind::Rust && pathed && near.locals.is_empty() && chain.len() == 1 {
            search::rust_scope_items(
                &a.text,
                self.line + 1,
                &chain[0],
                search::RustNamespace::Path,
            )
        } else {
            Vec::new()
        };
        let owners: Vec<String> = seen
            .iter()
            .map(|&l| {
                search::qualified(kind, &a.text, l, &chain[0]).unwrap_or_else(|| chain[0].clone())
            })
            .collect();
        let mine = match owners.as_slice() {
            [owner, rest @ ..] if rest.iter().all(|o| o == owner) => {
                let cut = self.truncated.get();
                let pattern = search::def_patterns(kind, &chain[0]).join("|");
                let elsewhere = self
                    .project_definitions(kind, &a.here, &chain[0], &pattern)
                    .iter()
                    .any(|h| h.path != a.here);
                self.truncated.set(cut);
                elsewhere.then(|| owner.clone())
            }
            _ => None,
        };
        let answer_files = mine.is_some().then(|| vec![a.here.clone()]);
        OwnerPath {
            sep,
            path,
            answer_files,
            mine,
            pathed,
            colons,
            class_method: false,
        }
    }

    fn owner_declared_once(&self, a: &Asked, near: &Near, q: &mut OwnerPath) -> bool {
        let (kind, chain) = (a.kind, &a.chain);
        let (path, sep) = (&q.path, q.sep);
        let owner = &chain[chain.len() - 1];
        let brings = Regex::new(&format!(
            r"^\s*(?:pub\s+)?us(?:e|ing)\b.*(?:\*|\bnamespace\b|\b{}\b)",
            regex::escape(owner)
        ))
        .expect("an escaped name keeps the pattern valid");
        let inside = |l: &str| ["crate", "self::", "super::"].iter().any(|k| l.contains(k));
        let outside = bound(&near.imports, &chain[0])
            .is_some_and(|p| !matches!(p[0].as_str(), "crate" | "self" | "super"))
            || self
                .buf
                .lines
                .iter()
                .any(|l| brings.is_match(l) && !inside(l))
            || a.lead
                .strip_suffix(&format!("{path}{sep}"))
                .is_some_and(|b| b.ends_with('\\'));
        let owners = search::def_patterns(kind, owner).join("|");
        let mut declared = self.project_definitions(kind, &a.here, owner, &owners);
        if kind == Kind::C {
            declared = search::c_type_rows(
                owner,
                declared,
                |p| self.text_of(p),
                search::CTypeWord::NAMED,
            );
            if search::cpp_one_namespace(owner, &declared, |p| self.text_of(p)) {
                declared.truncate(1);
            }
        }
        if !outside
            && declared.len() > 1
            && kind == Kind::Rust
            && chain.len() == 1
            && !declared.iter().any(|h| h.path == a.here)
        {
            let files = search::rust_use_files(&self.files, &a.here, &a.text, owner);
            q.answer_files = (!files.is_empty()).then_some(files);
        }
        self.truncated.set(false);
        !outside && (declared.len() == 1 || q.answer_files.is_some())
    }

    fn answered_by_path(&mut self, a: &Asked, p: &Patterns, q: &OwnerPath) -> bool {
        let (kind, chain, word) = (a.kind, &a.chain, &a.word);
        let mut hits = match &q.answer_files {
            Some(files) => self
                .grep(&p.pattern, false, false, |p| files.iter().any(|f| f == p))
                .unwrap_or_default(),
            None => self.project_definitions(kind, &a.here, word, &p.pattern),
        };
        if kind == Kind::Rust {
            let variants = self.rust_variants(&a.here, word);
            hits.extend(variants.into_iter().filter(|v| {
                q.answer_files
                    .as_ref()
                    .is_none_or(|files| files.contains(&v.path))
            }));
        }
        let singleton = q.class_method && word != "initialize";
        let concerns = singleton.then(|| hits.clone());
        let types = (kind == Kind::C).then(|| hits.clone());
        let named = match (self.path_named(a, q, hits, singleton), types) {
            (named, Some(types)) if named.is_empty() => self.cpp_path_named(a, q, types),
            (named, _) => named,
        };
        let mut named = self.named_kept(a, named);
        let owner = match (kind, chain.as_slice()) {
            (Kind::Jvm, [owner]) => self.jvm_owner_type(&a.here, &a.text, owner),
            _ => None,
        };
        if let Some(decl) = &owner {
            named.retain(|c| self.jvm_owns(decl, &c.hit, word));
        }
        if !named.is_empty() {
            return self.show_found(a, named);
        }
        if let (Kind::Jvm, [name]) = (kind, chain.as_slice()) {
            let found = self.enum_constant(kind, &a.here, name, word);
            if !found.is_empty() {
                return self.show_found(a, found);
            }
            if let Some(decl) = owner {
                if let Some(found) = self.jvm_owner_member(&a.here, decl, name, word) {
                    return self.show_found(a, found);
                }
                self.offer_only = true;
            }
        }
        if q.class_method {
            let (asked, found) =
                self.ruby_class_method_elsewhere(&a.here, chain, word, &p.pattern, concerns);
            self.show_definitions(kind, &asked, &a.here, found, None);
            return true;
        }
        self.truncated.set(false);
        self.offer_only |= q.colons;
        false
    }

    fn path_named(
        &self,
        a: &Asked,
        q: &OwnerPath,
        hits: Vec<Hit>,
        singleton: bool,
    ) -> Vec<Candidate> {
        let (kind, word, sep, path) = (a.kind, &a.word, q.sep, &q.path);
        let (answer_files, mine) = (&q.answer_files, &q.mine);
        let full = format!("{path}{sep}{word}");
        let starts_with_project_module = sep != "." && self.names_module(&a.chain[0]);
        hits.into_iter()
            .filter(|h| {
                let Some(t) = self.text_of(&h.path) else {
                    return false;
                };
                let qualified = search::qualified(kind, &t, h.line1, word);
                qualified.is_some_and(|q| match answer_files {
                    Some(files) if mine.is_some() => {
                        mine.as_ref()
                            .is_some_and(|o| q == format!("{o}{sep}{word}"))
                            && files.contains(&h.path)
                    }
                    Some(files) => q == full && files.contains(&h.path),
                    None => {
                        q == full
                            || q.ends_with(&format!("{sep}{full}"))
                            || (starts_with_project_module && full.ends_with(&format!("{sep}{q}")))
                    }
                }) && (!singleton || search::ruby_singleton(&t, h.line1))
            })
            .map(|hit| Candidate {
                reason: match answer_files {
                    Some(_) if mine.is_some() => Reason::Path(path.clone()),
                    Some(_) => Reason::Import(hit.path.display().to_string()),
                    None => Reason::Path(path.clone()),
                },
                hit,
            })
            .collect()
    }

    fn cpp_path_named(&self, a: &Asked, q: &OwnerPath, hits: Vec<Hit>) -> Vec<Candidate> {
        let word = &a.word;
        let scoped: Vec<(Hit, Option<String>)> = hits
            .into_iter()
            .map(|h| {
                let name = Self::c_type_line(word, &h)
                    .then(|| self.text_of(&h.path))
                    .flatten()
                    .and_then(|t| search::cpp_scoped_name(&t, h.line1, word));
                (h, name)
            })
            .collect();
        let pick = |want: &str| -> Vec<Candidate> {
            (scoped.iter())
                .filter(|(_, name)| name.as_deref() == Some(want))
                .map(|(hit, _)| Candidate {
                    hit: hit.clone(),
                    reason: Reason::Path(q.path.clone()),
                })
                .collect()
        };
        let pos = (a.text.split('\n').take(self.line))
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + a.range.start;
        let path = match search::cpp_alias_in_scope(&a.text, pos, &a.chain[0]) {
            Some(real) => [&real[..], &a.chain[1..]].concat().join("::"),
            None => q.path.clone(),
        };
        pick(&format!("{path}::{word}"))
    }

    fn named_kept(&self, a: &Asked, named: Vec<Candidate>) -> Vec<Candidate> {
        let (kind, word) = (a.kind, &a.word);
        let on_row = named
            .iter()
            .any(|c| c.hit.path == a.here && c.hit.line1 == self.line + 1);
        let named = if kind == Kind::C && !on_row {
            let rows: Vec<Hit> = named.iter().map(|c| c.hit.clone()).collect();
            let kept = search::c_type_rows(
                word,
                rows,
                |_| None,
                search::CTypeWord {
                    constructs: true,
                    struct_tag: false,
                },
            );
            named
                .into_iter()
                .filter(|c| {
                    kept.iter()
                        .any(|h| h.path == c.hit.path && h.line1 == c.hit.line1)
                })
                .collect()
        } else {
            named
        };
        if kind == Kind::Swift
            && named
                .iter()
                .any(|c| !search::swift_instance_member(&c.hit.text))
        {
            named
                .into_iter()
                .filter(|c| !search::swift_instance_member(&c.hit.text))
                .collect()
        } else {
            named
        }
    }

    pub(super) fn answered_by_elixir(&mut self, a: &Asked, near: &Near) -> bool {
        let (kind, dotted, chain, word) = (a.kind, a.dotted, &a.chain, &a.word);
        if kind == Kind::Elixir && dotted && !chain.is_empty() && near.locals.is_empty() {
            let found = self
                .external_definitions(kind, word, chain, dotted, &near.imports, true)
                .unwrap_or_default();
            if !found.is_empty() {
                return self.show_found(a, found);
            }
        }
        let attribute = a.before.ends_with('@');
        let imported = !attribute
            && a.text.lines().any(|l| {
                l.trim_start().starts_with("import ")
                    && (!l.contains("only:") || l.contains(&format!("{word}:")))
            });
        let module_pattern = search::def_patterns(kind, word)
            .into_iter()
            .filter(|p| {
                if attribute {
                    p.starts_with(r"^\s*@")
                } else {
                    !p.starts_with(r"^\s*@") && !p.contains("defstruct")
                }
            })
            .collect::<Vec<_>>()
            .join("|");
        if a.elixir_call()
            && !imported
            && let Ok(re) = Regex::new(&module_pattern)
        {
            let lines = search::elixir_module_lines(&a.text, self.line + 1, &re);
            if !lines.is_empty() && !lines.contains(&(self.line + 1)) {
                let found = self.lines_here(a, lines, &Reason::File);
                return self.show_found(a, found);
            }
        }
        false
    }

    pub(super) fn answered_by_cs_path(&mut self, a: &Asked, near: &Near, q: &OwnerPath) -> bool {
        if a.kind == Kind::CSharp
            && q.pathed
            && near.locals.is_empty()
            && let Some(found) = self.cs_enum_member(&a.here, &a.text, &a.chain, &a.word, &q.path)
        {
            return self.show_found(a, found);
        }
        if a.kind == Kind::CSharp
            && a.dotted
            && near.locals.is_empty()
            && let Some(first) = a.chain.first()
            && first.starts_with(|c: char| c.is_ascii_uppercase())
            && !self.cs_declares(&a.here, first)
        {
            return self.show_found(a, Vec::new());
        }
        false
    }
}
