//! The declarations a type owns: its own, its package's, and the ones it inherits.

use super::*;

impl App {
    /// The one top-level declaration `parts` names as `file` sees it: in the file itself (for Go,
    /// its package), else in the project module an import binds the first part to. `None` when
    /// there is none or more than one, and for a module outside the project.
    pub(super) fn declaration(&self, kind: Kind, file: &Path, parts: &[String]) -> Option<Hit> {
        let (name, chain) = parts.split_last()?;
        let one = |hits: Vec<Hit>| <[Hit; 1]>::try_from(hits).ok().map(|[hit]| hit);
        // TypeScript's `Outer.Inner.Widget` may be behind namespaces of the file itself (#100).
        if chain.is_empty() || kind == Kind::TsJs {
            let within = (!chain.is_empty()).then(|| parts.join("."));
            let hits = self.package_declarations(kind, file, name, within.as_deref());
            if !hits.is_empty() {
                return one(self.host_built(kind, hits));
            }
        }
        let imports = search::imports(kind, &self.text_of(file)?);
        let path = bound(&imports, chain.first().unwrap_or(name))?;
        let found = self.imported_definitions(kind, file, name, chain, &path)?;
        one(found.into_iter().map(|c| c.hit).collect())
    }

    /// The declarations of `name` that `file` sees without an import: at the top level, or
    /// `within` the namespaces a TypeScript path spells (`Outer.Inner`).
    pub(super) fn package_declarations(
        &self,
        kind: Kind,
        file: &Path,
        name: &str,
        within: Option<&str>,
    ) -> Vec<Hit> {
        let own = self.package_files(kind, file);
        let pattern = search::def_patterns(kind, name).join("|");
        self.grep(&pattern, false, false, |p| own.iter().any(|f| f == p))
            .unwrap_or_default()
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|text| {
                    search::qualified(kind, &text, h.line, name).as_deref() == within
                })
            })
            .collect()
    }

    /// The files a name of `file` is declared in without an import: the file, or for Go every
    /// file of its package (a `_test.go` file only from a test).
    pub(super) fn package_files(&self, kind: Kind, file: &Path) -> Vec<PathBuf> {
        let mut files = vec![file.to_path_buf()];
        if kind == Kind::Go {
            let test = |f: &Path| f.to_string_lossy().ends_with("_test.go");
            files.extend(
                self.files
                    .iter()
                    .filter(|f| f.parent() == file.parent() && f.as_path() != file)
                    .filter(|f| search::kind_of(f) == Some(Kind::Go) && (test(file) || !test(f)))
                    .cloned(),
            );
        }
        files
    }

    /// The declarations of `word` inside `ty` itself: its methods, Go's methods on it in its
    /// package and the method lines of a Go interface, named `Type.word` by
    /// [`search::qualified`].
    pub(super) fn members_of(&self, kind: Kind, ty: &Typed, word: &str) -> Vec<Hit> {
        let Some(text) = self.text_of(&ty.path) else {
            return Vec::new();
        };
        let owner = search::qualified(kind, &text, ty.line, &ty.name).unwrap_or(ty.name.clone());
        let want = Some(format!("{owner}.{word}"));
        let patterns = search::member_or_signature(kind, word).unwrap_or_default();
        let files = self.package_files(kind, &ty.path);
        let hits = self
            .grep(&patterns.join("|"), false, false, |p| {
                files.iter().any(|f| f == p)
            })
            .unwrap_or_default()
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path)
                    .and_then(|text| search::qualified(kind, &text, h.line, word))
                    == want
            })
            .collect();
        self.host_built(kind, hits)
    }

    /// Of several Go declarations of one name, those in files the host's `go build` compiles
    /// (#100): `Clock` of `clock_linux.go` and of `clock_windows.go` is one type per platform.
    /// A tag of the project's own (`gogit`) is unset, as it is for a plain `go build`, unless
    /// `GOFLAGS` sets it (#137). Only on certainty: every file is known to be built or known not
    /// to be ([`search::go_built`]), so a constraint the rules do not read leaves all of them.
    /// Asked from the open file, which the host must not be known to skip: inside
    /// `clock_windows.go` on another host, or inside `repo_gogit.go`, nothing is preferred.
    pub(super) fn host_built(&self, kind: Kind, hits: Vec<Hit>) -> Vec<Hit> {
        if kind != Kind::Go || hits.len() < 2 {
            return hits;
        }
        let built = |p: &Path| {
            self.text_of(p)
                .and_then(|t| search::go_built(p, &t, &self.go_build))
        };
        if self
            .rel_current()
            .is_some_and(|here| built(&here) == Some(false))
        {
            return hits;
        }
        let known: Option<Vec<bool>> = hits.iter().map(|h| built(&h.path)).collect();
        let Some(known) = known.filter(|k| k.contains(&true)) else {
            return hits;
        };
        let mut keep = known.into_iter();
        hits.into_iter()
            .filter(|_| keep.next() == Some(true))
            .collect()
    }

    /// The line on which `ty` itself declares the field `word` (#104), [`search::field_line`]: a
    /// class-body annotation, a constructor parameter, a struct field, or with `assigned` the
    /// first `self.word = …` of a type that declares it no other way.
    pub(super) fn field_of(
        &self,
        kind: Kind,
        ty: &Typed,
        word: &str,
        assigned: bool,
    ) -> Option<Hit> {
        let text = self.text_of(&ty.path)?;
        let (line, is) = search::field_line(kind, &text, ty.line, word)?;
        (is == assigned).then(|| Hit {
            path: ty.path.clone(),
            line,
            text: text.lines().nth(line - 1).unwrap_or_default().to_owned(),
        })
    }

    /// The project's declarations of `word` by name as a member of any type: the lines `members`
    /// matches, and each type's declaration of a field `word` ([`search::field_rows`]), so a type
    /// is one row and a local or a literal's key of that name is none (#104). The field lines are
    /// grepped apart: their patterns match object keys, locals and keyword arguments too, which
    /// must not push a method past [`search::MAX_HITS`], and a cut there is kept for the count.
    pub(super) fn members_by_name(
        &mut self,
        kind: Kind,
        here: &Path,
        word: &str,
        members: &str,
    ) -> Vec<Hit> {
        let mut hits = self.project_definitions(kind, here, word, members);
        let Some(fields) = search::field_patterns(kind, word) else {
            return hits;
        };
        let raw = self.project_definitions(kind, here, word, &fields.join("|"));
        let mut by_file: Vec<(PathBuf, Vec<usize>)> = Vec::new();
        for h in raw {
            match by_file.last_mut() {
                Some((path, lines)) if *path == h.path => lines.push(h.line),
                _ => by_file.push((h.path, vec![h.line])),
            }
        }
        for (path, lines) in by_file {
            let Some(text) = self.text_of(&path) else {
                continue;
            };
            for line in search::field_rows(kind, &text, &lines, word) {
                if !hits.iter().any(|h| h.path == path && h.line == line) {
                    hits.push(Hit {
                        text: text.lines().nth(line - 1).unwrap_or_default().to_owned(),
                        path: path.clone(),
                        line,
                    });
                }
            }
        }
        let current = self.rel_current();
        hits.sort_by_cached_key(|h| (current.as_ref() != Some(&h.path), h.path.clone(), h.line));
        hits
    }

    /// What a field's own declaration offers (#104): the declarations of its name as a member of
    /// any type, by name, the declaration itself among them.
    pub(super) fn field_namesakes(
        &mut self,
        kind: Kind,
        here: &Path,
        word: &str,
    ) -> Vec<Candidate> {
        let members = search::member_patterns(kind, word)
            .unwrap_or_default()
            .join("|");
        self.members_by_name(kind, here, word, &members)
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    /// #68 step 6. What implements the member the cursor stands on: the same member in the types
    /// that implement the interface, protocol, abstract or base class declaring it. Empty when
    /// the cursor is not on such a declaration, and when nothing implements it, and `d` then goes
    /// on as before.
    ///
    /// A Python class and a TypeScript class or interface implement another by naming it, so the
    /// subtypes are walked down from the type ([`Self::subtype_impls`]). A Go type and a Python
    /// `Protocol` implementer name nothing, so there the rule is structural: a member of the same
    /// name taking the same number of parameters, which is what Go's implicit interfaces and a
    /// protocol ask for. Only the project is searched: an interface is opened to find what this
    /// project does with it.
    pub(super) fn implementations(
        &self,
        kind: Kind,
        here: &Path,
        text: &str,
        word: &str,
    ) -> Vec<Candidate> {
        if !matches!(kind, Kind::Python | Kind::TsJs | Kind::Go) {
            return Vec::new();
        }
        let line = self.line + 1;
        let declares = search::member_or_signature(kind, word)
            .and_then(|p| Regex::new(&p.join("|")).ok())
            .is_some_and(|re| re.is_match(self.line_str()));
        let Some(owner_line) = search::owner_decl(kind, text, line).filter(|_| declares) else {
            return Vec::new();
        };
        // `Notifier.send`, what the status line and every picker row name the member by.
        let Some(member) = search::qualified(kind, text, line, word) else {
            return Vec::new();
        };
        let owner = Typed {
            name: member.rsplit('.').nth(1).unwrap_or_default().to_owned(),
            path: here.to_path_buf(),
            line: owner_line,
        };
        let decl = text.lines().nth(owner_line - 1).unwrap_or_default();
        let structural = match kind {
            // A method beside its type is no interface method and has no implementations.
            Kind::Go => decl.contains("interface"),
            Kind::Python => is_protocol(text, owner_line),
            _ => false,
        };
        if owner.name.is_empty() || (kind == Kind::Go && !structural) {
            return Vec::new();
        }
        let hits = if structural {
            self.structural_impls(kind, here, word, text, line, owner_line)
        } else {
            self.subtype_impls(kind, here, word, &owner)
        };
        hits.into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Implementation(member.clone()),
            })
            .collect()
    }

    /// The member `word` in the types that name `owner` as a base, and in the types that name
    /// those: one grep of the project per level. A hit counts only when its own `extends`,
    /// `implements` or Python bases name a type already found and its file can see that name —
    /// it declares it, or an import binds it — so a same-named class in another package is no
    /// subtype. A type that inherits the member without declaring it is no implementation.
    fn subtype_impls(&self, kind: Kind, here: &Path, word: &str, owner: &Typed) -> Vec<Hit> {
        let mut names = vec![owner.name.clone()];
        let mut types = vec![(owner.name.clone(), owner.path.clone())];
        let mut seen = vec![(owner.path.clone(), owner.line)];
        let mut out = Vec::new();
        // ponytail: four levels of subtypes, one grep each. Deeper hierarchies want an index.
        for _ in 0..4 {
            let Some(pattern) = search::subtype_patterns(kind, &names) else {
                break;
            };
            let hits = self
                .grep(&pattern, false, false, |p| {
                    search::in_def_scope(kind, here, p)
                })
                .unwrap_or_default();
            let mut next = Vec::new();
            let mut found = Vec::new();
            for hit in hits {
                let Some(text) = self.text_of(&hit.path) else {
                    continue;
                };
                // The clause the grep matched may be one line of a header wrapped over
                // several, and the type is declared on the first of them.
                let Some(decl) = search::type_decl_at(kind, &text, hit.line) else {
                    continue;
                };
                let name = text
                    .lines()
                    .nth(decl - 1)
                    .and_then(|l| search::type_name(kind, l));
                let Some(name) = name else {
                    continue;
                };
                if seen.contains(&(hit.path.clone(), decl)) {
                    continue;
                }
                let declares = |text: &str, name: &str| {
                    text.lines()
                        .any(|l| search::type_name(kind, l).as_deref() == Some(name))
                };
                // The import may name another type of the same name: a module of the project
                // that declares one, in a file none of the types found so far lives in, looked
                // for behind a barrel too (#100).
                let another = |first: &str, module: &[String]| {
                    let module = match kind {
                        Kind::TsJs => &module[..module.len().saturating_sub(1)],
                        _ => module,
                    };
                    // A Python import may name a module or a name in one; a TypeScript one
                    // names its module whole, and a shorter path is another module.
                    let shortest = match kind {
                        Kind::TsJs => module.len(),
                        _ => 1,
                    };
                    (shortest..=module.len()).rev().any(|n| {
                        let files = self.behind_barrels(kind, &hit.path, &module[..n], first, 0);
                        let ours = |f: &PathBuf| types.iter().any(|(t, p)| t == first && p == f);
                        !files.iter().any(ours)
                            && files
                                .iter()
                                .any(|f| self.text_of(f).is_some_and(|t| declares(&t, first)))
                    })
                };
                let sees = |first: &str| {
                    if declares(&text, first) {
                        // Its own type of that name is ours only in the file ours is in.
                        return !types.iter().any(|(t, _)| t == first)
                            || types.iter().any(|(t, p)| t == first && *p == hit.path);
                    }
                    bound(&search::imports(kind, &text), first)
                        .is_some_and(|module| !another(first, &module))
                };
                // Read off the header's first line, wherever the hit is: Python's bases, and
                // TypeScript's, where the matched line alone may be a constraint of a type
                // parameter, `S extends Notifier,`.
                let derives = search::bases(kind, &text, decl)
                    .into_iter()
                    .chain(search::interfaces(kind, &text, decl))
                    .filter_map(|b| search::type_path(kind, &b))
                    .any(|p| p.last().is_some_and(|n| names.contains(n)) && sees(&p[0]));
                if !derives {
                    continue;
                }
                seen.push((hit.path.clone(), decl));
                found.push((name.clone(), hit.path.clone()));
                next.push(name);
                if let Some(at) = search::member_decl(kind, &text, decl, word) {
                    out.push(Hit {
                        text: text.lines().nth(at - 1).unwrap_or_default().to_owned(),
                        path: hit.path,
                        line: at,
                    });
                }
            }
            if next.is_empty() || out.len() >= search::MAX_HITS {
                break;
            }
            names = next;
            types = found;
        }
        out
    }

    /// The files of the project `module` names as `from` spells it, a TypeScript barrel among
    /// them replaced by the files it hands `name` on from (`export * from "./a"`), which may be
    /// barrels again. A barrel that leads nowhere in the project stays itself.
    fn behind_barrels(
        &self,
        kind: Kind,
        from: &Path,
        module: &[String],
        name: &str,
        depth: usize,
    ) -> Vec<PathBuf> {
        let files = search::module_files(kind, &self.root, &self.files, from, module);
        // ponytail: four barrels deep, which also ends two that export each other.
        if kind != Kind::TsJs || depth == 4 {
            return files;
        }
        files
            .into_iter()
            .flat_map(|f| {
                // A file that declares the name answers for itself, whatever else it hands on.
                let text = self.text_of(&f).filter(|t| {
                    !t.lines()
                        .any(|l| search::type_name(kind, l).as_deref() == Some(name))
                });
                let behind: Vec<PathBuf> = text
                    .map(|t| search::reexports(&t, name))
                    .unwrap_or_default()
                    .iter()
                    .flat_map(|m| self.behind_barrels(kind, &f, m, name, depth + 1))
                    .collect();
                match behind.is_empty() {
                    true => vec![f],
                    false => behind,
                }
            })
            .collect()
    }

    /// Every member of `word` the project declares with as many parameters as the one on `line`,
    /// outside the type declaring it: what implements a Go interface or a Python protocol, since
    /// an implementer of either names nothing. A Python protocol is answered by classes, so
    /// another protocol declaring the same member is not one of them.
    fn structural_impls(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        text: &str,
        line: usize,
        owner_line: usize,
    ) -> Vec<Hit> {
        let want = search::params(kind, text, line);
        let signature = search::go_signature(text, line);
        // A Go interface's own method lines carry no receiver: only the methods answer.
        let patterns = match kind {
            Kind::Go => search::member_patterns(kind, word),
            _ => search::member_or_signature(kind, word),
        };
        let (Some(want), Some(patterns)) = (want, patterns) else {
            return Vec::new();
        };
        self.project_definitions(kind, here, word, &patterns.join("|"))
            .into_iter()
            .filter(|h| h.line != line || h.path != here)
            .filter(|h| {
                let Some(text) = self.text_of(&h.path) else {
                    return false;
                };
                if search::params(kind, &text, h.line) != Some(want) {
                    return false;
                }
                // Go writes its types: the same number of parameters of other types, or another
                // result, implements nothing. Where either cannot be read, the count stands.
                if kind == Kind::Go
                    && let (Some(a), Some(b)) = (&signature, search::go_signature(&text, h.line))
                    && *a != b
                {
                    return false;
                }
                kind != Kind::Python
                    || search::owner_decl(kind, &text, h.line)
                        .filter(|&d| d != owner_line || h.path != here)
                        .is_some_and(|d| !is_protocol(&text, d))
            })
            .collect()
    }

    /// The declaration of `word` that `super` reaches from `ty`: a method, else a declared field,
    /// of the types above it. Under one base that is the nearest one up the line. Under several,
    /// Python orders them as no walk by depth does, so only what needs no ordering is proven: the
    /// first base declaring the member itself, or every base leading to the same line. `Err` when
    /// they differ, or when a base is outside the project and may declare the member first. The
    /// same holds at every level the answer is looked for, so a diamond or an outside base under
    /// the one direct base proves nothing either.
    pub(super) fn above(
        &self,
        kind: Kind,
        ty: &Typed,
        word: &str,
        depth: usize,
    ) -> Result<Vec<Hit>, String> {
        let broke = || "super".to_owned();
        let find = |t: &Typed| {
            let members = self.members_of(kind, t, word);
            match members.is_empty() {
                true => self.field_of(kind, t, word, false).map(|hit| vec![hit]),
                false => Some(members),
            }
        };
        // ponytail: eight levels up, which also ends a cycle.
        let text = self
            .text_of(&ty.path)
            .filter(|_| depth < 8)
            .ok_or_else(broke)?;
        // ponytail: bases that declare no member worth a jump, by their usual spelling.
        let plain = |b: &str| {
            let b = b.split('[').next().unwrap_or(b);
            let b = b.rsplit('.').next().unwrap_or(b);
            matches!(b, "Generic" | "Protocol" | "ABC" | "object")
        };
        let written = search::bases(kind, &text, ty.line);
        let bases: Vec<&String> = written.iter().filter(|b| !plain(b)).collect();
        let mut answers: Vec<Vec<Hit>> = Vec::new();
        for (i, base) in bases.iter().enumerate() {
            let Some(base) = self.type_decl(kind, &ty.path, base) else {
                return Err(broke());
            };
            let hits = match find(&base) {
                Some(own) if i == 0 => return Ok(own),
                Some(own) => own,
                None => self.above(kind, &base, word, depth + 1)?,
            };
            let lines = |hits: &[Hit]| -> Vec<(PathBuf, usize)> {
                hits.iter().map(|h| (h.path.clone(), h.line)).collect()
            };
            if !hits.is_empty() && answers.iter().all(|a| lines(a) != lines(&hits)) {
                answers.push(hits);
            }
        }
        match answers.len() {
            0 | 1 => Ok(answers.pop().unwrap_or_default()),
            _ => Err(broke()),
        }
    }

    /// `find` in `ty`, then in the types it extends or embeds, nearest first: the first answer.
    pub(super) fn hierarchy<R>(
        &self,
        kind: Kind,
        ty: &Typed,
        depth: usize,
        find: &mut dyn FnMut(&Typed) -> Option<R>,
    ) -> Option<R> {
        if let Some(found) = find(ty) {
            return Some(found);
        }
        // ponytail: eight levels up, which also ends a cycle.
        if depth == 8 {
            return None;
        }
        let text = self.text_of(&ty.path)?;
        search::bases(kind, &text, ty.line)
            .iter()
            .filter_map(|base| self.type_decl(kind, &ty.path, base))
            .find_map(|base| self.hierarchy(kind, &base, depth + 1, &mut *find))
    }
}

/// Whether the Python class declared on 1-based `decl` of `text` is a `typing.Protocol`, which
/// anything with its members implements without naming it.
fn is_protocol(text: &str, decl: usize) -> bool {
    search::bases(Kind::Python, text, decl)
        .iter()
        .filter_map(|b| search::type_path(Kind::Python, b))
        .any(|p| p.last().is_some_and(|n| n == "Protocol"))
}
