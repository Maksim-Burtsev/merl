use super::{App, Candidate, HashSet, Hit, Kind, Path, Reason, search};

impl App {
    pub(super) fn swift_by_labels(
        &self,
        name: &str,
        after: usize,
        found: &mut Vec<Candidate>,
    ) -> bool {
        let lines: Vec<&str> = self.buf.lines.iter().map(String::as_str).collect();
        let Some(call) = search::swift_call_labels(&lines, self.line, after) else {
            return false;
        };
        let labels: Option<Vec<Vec<search::SwiftParam>>> = found
            .iter()
            .map(|c| {
                let text = self.text_of(&c.hit.path)?;
                let lines: Vec<&str> = text.lines().collect();
                search::swift_param_labels(&lines, c.hit.line1.checked_sub(1)?, name)
            })
            .collect();
        let Some(labels) = labels else {
            return false;
        };
        let fit = labels.iter().any(|l| search::swift_call_fits(&call, l));
        if fit {
            let mut fits = labels.iter().map(|l| search::swift_call_fits(&call, l));
            found.retain(|_| fits.next().unwrap_or(true));
            let kept: Vec<_> = labels
                .into_iter()
                .filter(|l| search::swift_call_fits(&call, l))
                .collect();
            self.swift_by_argument_types(&call, &kept, found);
        }
        fit
    }

    pub(super) fn swift_one_owner(&self, name: &str, found: &[Candidate]) -> bool {
        let owners: Option<Vec<String>> = found
            .iter()
            .map(|c| {
                let text = self.text_of(&c.hit.path)?;
                let q = search::qualified(Kind::Swift, &text, c.hit.line1, name)?;
                Some(q.strip_suffix(name)?.to_owned())
            })
            .collect();
        owners.is_some_and(|o| o.len() > 1 && o.iter().all(|x| *x == o[0] && !x.is_empty()))
    }

    pub(super) fn swift_init_by_labels(
        &self,
        here: &Path,
        word: &str,
        after: usize,
        found: &mut Vec<Candidate>,
    ) {
        let [c] = found.as_slice() else {
            return;
        };
        let typed = search::swift_type_header(&c.hit.text)
            .is_some_and(|h| h.name == word && h.keyword != "protocol" && h.keyword != "extension");
        let Some(owner) = self.text_of(&c.hit.path).filter(|_| typed).map(|t| {
            search::qualified(Kind::Swift, &t, c.hit.line1, word).unwrap_or_else(|| word.to_owned())
        }) else {
            return;
        };
        let full = format!("{owner}.init");
        let cut = self.truncated.get();
        let pattern = search::def_patterns(Kind::Swift, "init").join("|");
        let mut inits: Vec<Candidate> = self
            .project_definitions(Kind::Swift, here, "init", &pattern)
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path)
                    .and_then(|t| search::qualified(Kind::Swift, &t, h.line1, "init"))
                    .is_some_and(|q| q == full || q.ends_with(&format!(".{full}")))
            })
            .map(|hit| Candidate {
                hit,
                reason: c.reason.clone(),
            })
            .collect();
        self.truncated.set(cut);
        if !inits.is_empty() && self.swift_by_labels("init", after, &mut inits) && inits.len() == 1
        {
            *found = inits;
        }
    }
    pub(super) fn swift_self_members(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        pattern: &str,
    ) -> Option<Vec<Candidate>> {
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let at = search::swift_enclosing_type(&lines, &literal, self.line + 1)?;
        let search::SwiftTypeHeader {
            mut keyword,
            name: own,
            first_inherited: mut base,
            where_constrained: constrained,
        } = search::swift_type_header(lines[at - 1])?;
        if constrained {
            return None;
        }
        let cut = self.truncated.get();
        let hits = self.project_definitions(Kind::Swift, here, word, pattern);
        let mut ty = own.clone();
        let mut seen = HashSet::from([own.clone()]);
        let mut nearest: Option<Vec<Candidate>> = None;
        loop {
            let full = format!("{ty}.{word}");
            let found: Vec<Candidate> = hits
                .iter()
                .filter(|h| {
                    self.text_of(&h.path)
                        .and_then(|t| search::qualified(Kind::Swift, &t, h.line1, word))
                        .is_some_and(|q| q == full || q.ends_with(&format!(".{full}")))
                })
                .map(|h| Candidate {
                    hit: h.clone(),
                    reason: Reason::Receiver(format!("self: {own}")),
                })
                .collect();
            if !found.is_empty() {
                let rows: Vec<Hit> = found.iter().map(|c| c.hit.clone()).collect();
                if !self.swift_misfit(word, &rows) {
                    return Some(found);
                }
                nearest.get_or_insert(found);
            }
            let next = match (keyword.as_str(), base) {
                ("class", Some(next)) if seen.insert(next.clone()) => next,
                _ => {
                    self.truncated.set(cut);
                    return nearest.or_else(|| self.swift_self_default(here, &own, word));
                }
            };
            let classes: Vec<_> = self
                .project_definitions(
                    Kind::Swift,
                    here,
                    &next,
                    &search::def_patterns(Kind::Swift, &next).join("|"),
                )
                .iter()
                .filter_map(|h| search::swift_type_header(&h.text))
                .filter(|h| h.keyword == "class" && h.name == next)
                .collect();
            let [
                search::SwiftTypeHeader {
                    keyword: k,
                    first_inherited: b,
                    ..
                },
            ] = classes.as_slice()
            else {
                self.truncated.set(cut);
                return nearest.or_else(|| self.swift_self_default(here, &own, word));
            };
            (keyword, base, ty) = (k.clone(), b.clone(), next);
        }
    }

    pub(super) fn swift_unseen(&self, here: &Path, hits: &mut Vec<Hit>) {
        let tests = search::swift_test_dirs(&self.root);
        let in_tests = |p: &Path| {
            let p = p.strip_prefix(&self.root).unwrap_or(p);
            tests.iter().any(|t| p.starts_with(t))
        };
        let testing = in_tests(here);
        let literal = search::literal_lines(Kind::Swift, &self.buf.lines.join("\n"));
        hits.retain(|h| {
            if h.deleted.is_some() {
                return true;
            }
            let own = h.path == here;
            if (!testing && in_tests(&h.path)) || (!own && search::swift_file_private(&h.text)) {
                return false;
            }
            let place = (search::swift_type_decl(&h.text) || search::swift_func_decl(&h.text))
                .then(|| self.text_of(&h.path))
                .flatten()
                .and_then(|t| search::swift_type_place(&t, h.line1));
            match place {
                Some(search::SwiftTypePlace::FunctionBody { header_line1: f }) => {
                    own && search::swift_within(&self.buf.lines, &literal, self.line + 1, f)
                }
                _ => true,
            }
        });
        let Some(mine) = tests
            .iter()
            .find(|t| here.strip_prefix(&self.root).unwrap_or(here).starts_with(t))
        else {
            return;
        };
        let signature = |h: &Hit| {
            let text = self.text_of(&h.path)?;
            let lines: Vec<&str> = text.lines().collect();
            let literal = search::literal_lines(Kind::Swift, &text);
            let at = search::swift_scope(&lines, &literal, h.line1).header_line1;
            let mut decl: Vec<&str> = vec![lines.get(at.checked_sub(1)?)?.trim()];
            for line in lines.get(h.line1.checked_sub(1)?..)?.iter().take(20) {
                decl.push(line.trim());
                if line.contains('{') {
                    return Some(decl.join("\n"));
                }
            }
            None
        };
        let own: Vec<String> = hits
            .iter()
            .filter(|h| {
                h.path
                    .strip_prefix(&self.root)
                    .unwrap_or(&h.path)
                    .starts_with(mine)
            })
            .filter_map(signature)
            .collect();
        hits.retain(|h| in_tests(&h.path) || signature(h).is_none_or(|s| !own.contains(&s)));
    }

    pub(super) fn swift_uncalled(&self, word: &str, after: usize, found: &mut Vec<Candidate>) {
        let rest = self
            .line_str()
            .get(after..)
            .unwrap_or_default()
            .trim_start();
        let next = match rest {
            "" => self
                .buf
                .lines
                .get(self.line + 1)
                .map_or("", |l| l.trim_start()),
            _ => rest,
        };
        if next.trim_start_matches(['?', '!']).starts_with(['(', '{']) {
            return;
        }
        let func = |c: &Candidate| search::swift_declares_func(&c.hit.text);
        let owner = |c: &Candidate| {
            self.text_of(&c.hit.path)
                .and_then(|t| search::qualified(Kind::Swift, &t, c.hit.line1, word))
        };
        let properties: Vec<String> = found
            .iter()
            .filter(|c| func(c) == Some(false))
            .filter_map(owner)
            .collect();
        found
            .retain(|c| func(c) != Some(true) || owner(c).is_none_or(|o| !properties.contains(&o)));
    }

    fn swift_qualified(&self, h: &Hit, name: &str) -> String {
        self.text_of(&h.path)
            .and_then(|t| search::qualified(Kind::Swift, &t, h.line1, name))
            .unwrap_or_else(|| name.to_owned())
    }

    fn swift_defs(&self, here: &Path, name: &str, full: &str, exact: bool) -> Vec<Hit> {
        let pattern = search::def_patterns(Kind::Swift, name).join("|");
        let suffix = format!(".{full}");
        self.project_definitions(Kind::Swift, here, name, &pattern)
            .into_iter()
            .filter(|h| {
                self.in_code(Kind::Swift, h) && {
                    let q = self.swift_qualified(h, name);
                    q == full || (!exact && q.ends_with(&suffix))
                }
            })
            .collect()
    }

    pub(super) fn swift_argument_rows(
        &self,
        here: &Path,
        lead: &str,
        word: &str,
    ) -> Option<Result<Vec<Candidate>, String>> {
        let arg = search::swift_argument(lead)?;
        let cut = self.truncated.get();
        let types = self.swift_argument_types(here, &arg);
        let rows = types.as_ref().and_then(|types| {
            let mut rows: Vec<Hit> = Vec::new();
            for (scope, ty) in types {
                for h in self.swift_implicit_members(here, scope, ty, word)? {
                    if !rows.iter().any(|r| r.path == h.path && r.line1 == h.line1) {
                        rows.push(h);
                    }
                }
            }
            (!rows.is_empty()).then_some(rows)
        });
        let outside = match types.as_deref() {
            Some([(_, ty), rest @ ..]) if rows.is_none() && rest.iter().all(|(_, t)| t == ty) => ty
                .rsplit_once('.')
                .filter(|(_, last)| self.swift_defs(here, last, ty, false).is_empty())
                .map(|_| ty.clone()),
            _ => None,
        };
        self.truncated.set(cut);
        if let Some(ty) = outside {
            return Some(Err(ty));
        }
        let label = format!(".{word}: {}.{}({}:)", arg.owner, arg.callee, arg.label);
        Some(Ok(rows?
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Receiver(label.clone()),
            })
            .collect()))
    }

    pub(super) fn swift_implicit_init_rows(
        &mut self,
        here: &Path,
        word: &str,
        line0: usize,
        lead: &str,
    ) -> Vec<Candidate> {
        let Some(arg) = search::swift_argument(lead) else {
            return Vec::new();
        };
        let line = std::mem::replace(&mut self.line, line0);
        let cut = self.truncated.get();
        let rows = self.swift_init_rows(here, word, &arg);
        self.truncated.set(cut);
        self.line = line;
        rows.unwrap_or_default()
    }

    fn swift_init_rows(
        &self,
        here: &Path,
        word: &str,
        arg: &search::SwiftArgument,
    ) -> Option<Vec<Candidate>> {
        if arg.on_value {
            return None;
        }
        let types = self.swift_argument_types(here, arg)?;
        let [(scope, ty)] = types.as_slice() else {
            return None;
        };
        let last = ty.rsplit('.').next()?;
        let [d] = self.swift_defs(here, last, last, false).try_into().ok()?;
        let keyword = search::swift_type_header(&d.text)?.keyword;
        let of = self.swift_qualified(&d, last);
        let prefix = of.strip_suffix(ty.as_str())?.trim_end_matches('.');
        let seen = prefix.is_empty() || scope == prefix || scope.starts_with(&format!("{prefix}."));
        if !seen || !matches!(keyword.as_str(), "struct" | "class" | "enum" | "actor") {
            return None;
        }
        let text = self.text_of(&d.path)?;
        let rows = search::label_lines(
            Kind::Swift,
            &text,
            d.line1,
            last,
            word,
            &search::Owner::Args,
        )
        .into_iter()
        .map(|l| {
            let line1 = search::swift_signature_line(&text, l);
            Candidate {
                hit: Hit {
                    deleted: None,
                    path: d.path.clone(),
                    line1,
                    byte_col: None,
                    text: text.lines().nth(line1 - 1).unwrap_or_default().to_owned(),
                },
                reason: Reason::Label(format!("parameter of {of}")),
            }
        })
        .collect();
        Some(rows)
    }

    fn swift_argument_types(
        &self,
        here: &Path,
        arg: &search::SwiftArgument,
    ) -> Option<Vec<(String, String)>> {
        let hits = if arg.on_value {
            let chain: Vec<String> = arg.owner.split('.').map(str::to_owned).collect();
            let text = self.buf.lines.join("\n");
            let rows = self.swift_receiver_rows(here, &text, &chain, &arg.callee)?;
            if rows
                .iter()
                .any(|h| !h.text.contains("func ") || !search::swift_instance_member(&h.text))
            {
                return None;
            }
            rows
        } else {
            let full = format!("{}.{}", arg.owner, arg.callee);
            self.swift_defs(here, &arg.callee, &full, false)
        };
        let mut decls: Vec<Candidate> = hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Local,
            })
            .collect();
        if decls.is_empty() {
            return (!arg.on_value)
                .then(|| self.swift_memberwise_type(here, arg))?
                .map(|t| vec![t]);
        }
        if !self.swift_by_labels(&arg.callee, arg.open, &mut decls) {
            return None;
        }
        decls
            .iter()
            .map(|c| {
                let text = self.text_of(&c.hit.path)?;
                let lines: Vec<&str> = text.lines().collect();
                let params = search::swift_param_labels(&lines, c.hit.line1 - 1, &arg.callee)?;
                let mut named = params.iter().filter(|p| p.label == arg.label);
                let p = match (arg.label.as_str(), named.next(), named.next()) {
                    ("_", ..) => params.first().filter(|p| p.label == "_")?,
                    (_, Some(p), None) => p,
                    _ => return None,
                };
                let ty = p.ty.trim_end_matches(['?', '!']).to_owned();
                let first = ty.split('.').next()?.to_owned();
                let literal = search::literal_lines(Kind::Swift, &text);
                let mut at = c.hit.line1;
                while at > 0 {
                    if search::swift_generics(lines.get(at - 1)?).contains(&first) {
                        return None;
                    }
                    at = search::swift_scope(&lines, &literal, at).header_line1;
                }
                let scope = self.swift_qualified(&c.hit, &arg.callee);
                let scope = scope.strip_suffix(&arg.callee)?.trim_end_matches('.');
                Some((scope.to_owned(), ty))
            })
            .collect()
    }

    fn swift_memberwise_type(
        &self,
        here: &Path,
        arg: &search::SwiftArgument,
    ) -> Option<(String, String)> {
        let last = arg.owner.rsplit('.').next()?;
        let mut structs = self.swift_defs(here, last, &arg.owner, false);
        structs.retain(|h| {
            search::swift_type_header(&h.text).is_some_and(|t| t.keyword != "extension")
        });
        let [s] = structs.as_slice() else {
            return None;
        };
        if search::swift_type_header(&s.text)?.keyword != "struct" || arg.callee != "init" {
            return None;
        }
        let owner = self.swift_qualified(s, last);
        let props = self.swift_defs(here, &arg.label, &format!("{owner}.{}", arg.label), true);
        let [p] = props.as_slice() else {
            return None;
        };
        let ty = match search::swift_given(&p.text, &arg.label)? {
            search::SwiftGiven::Type(ty) => ty.trim_end_matches(['?', '!']).to_owned(),
            search::SwiftGiven::Value(v) if search::swift_var(&p.text, &arg.label) => {
                let (ty, case) = search::swift_case_value(&v)?;
                let rows = self.swift_implicit_members(here, &owner, &ty, &case)?;
                rows.iter()
                    .all(|h| search::swift_case(&h.text))
                    .then_some(ty)?
            }
            _ => return None,
        };
        (search::swift_instance_member(&p.text)).then_some((owner, ty))
    }

    fn swift_implicit_members(
        &self,
        here: &Path,
        scope: &str,
        ty: &str,
        word: &str,
    ) -> Option<Vec<Hit>> {
        let plain = ty.split('.').all(|p| {
            p.starts_with(char::is_uppercase) && p.chars().all(|c| c.is_alphanumeric() || c == '_')
        });
        let last = ty.rsplit('.').next().filter(|_| plain)?;
        let mut scope = scope.to_owned();
        loop {
            let full = if scope.is_empty() {
                ty.to_owned()
            } else {
                format!("{scope}.{ty}")
            };
            let types = self.swift_defs(here, last, &full, true);
            if !types.is_empty() {
                let keywords: Option<Vec<String>> = types
                    .iter()
                    .map(|h| search::swift_type_header(&h.text).map(|t| t.keyword))
                    .collect();
                let declared = keywords.is_some_and(|k| {
                    k.iter().all(|k| k != "protocol") && k.iter().any(|k| k != "extension")
                });
                let rows: Vec<Hit> = self
                    .swift_defs(here, word, &format!("{full}.{word}"), true)
                    .into_iter()
                    .filter(|h| {
                        search::swift_case(&h.text)
                            || search::swift_member_kind(&h.text)
                                == Some(search::SwiftMember::StaticOrClass)
                    })
                    .collect();
                return (declared && !rows.is_empty()).then_some(rows);
            }
            if scope.is_empty() {
                return None;
            }
            scope = scope.rsplit_once('.').map_or("", |(a, _)| a).to_owned();
        }
    }

    pub(super) fn swift_nested_unseen(&self, hits: &mut Vec<Hit>) {
        let literal = search::literal_lines(Kind::Swift, &self.buf.lines.join("\n"));
        hits.retain(|h| {
            let place = (h.deleted.is_none() && search::swift_type_decl(&h.text))
                .then(|| self.text_of(&h.path))
                .flatten()
                .and_then(|t| search::swift_type_place(&t, h.line1));
            match place {
                Some(search::SwiftTypePlace::Nested(outer)) => {
                    search::swift_sees_nested(&self.buf.lines, &literal, self.line + 1, &outer)
                }
                _ => true,
            }
        });
    }
}

impl App {
    pub(super) fn swift_attribute_types(&self, here: &Path, text: &str, hits: &mut Vec<Hit>) {
        hits.retain(|h| search::swift_type_decl(&h.text));
        self.swift_imported_modules(here, text, hits);
    }

    pub(super) fn swift_imported_modules(&self, here: &Path, text: &str, hits: &mut Vec<Hit>) {
        static IMPORT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(r"(?m)^[ \t]*(?:@\w+[ \t]+)*import[ \t]+(?:(?:class|struct|enum|protocol|func|var|let|typealias)[ \t]+)?(\w+)")
                .expect("the regex is a literal")
        });
        let targets = self.swift_package_targets();
        let module = |p: &Path| {
            let p = p.strip_prefix(&self.root).unwrap_or(p);
            targets
                .iter()
                .filter(|t| p.starts_with(&t.dir))
                .max_by_key(|t| t.dir.components().count())
                .map(|t| t.name.as_str())
                .filter(|n| n.chars().all(|c| c.is_alphanumeric() || c == '_'))
        };
        let Some(own) = module(here) else {
            return;
        };
        let imported: Vec<&str> = IMPORT
            .captures_iter(text)
            .filter_map(|c| Some(c.get(1)?.as_str()))
            .collect();
        let exported = |m: &str| {
            self.grep(
                &format!(r"@_exported\s+import\s+(\w+\s+)?{m}\b"),
                false,
                false,
                |p| p.extension().is_some_and(|e| e == "swift"),
            )
            .map_or(true, |found| !found.is_empty())
        };
        hits.retain(|h| match module(&h.path) {
            Some(m) if m != own => imported.contains(&m) || exported(m),
            _ => true,
        });
    }

    fn swift_package_targets(&self) -> Vec<search::SwiftTarget> {
        let xcode = std::fs::read_dir(&self.root).map_or(true, |d| {
            d.flatten()
                .any(|e| e.path().extension().is_some_and(|x| x == "xcodeproj"))
        });
        match std::fs::read_to_string(self.root.join("Package.swift")) {
            Ok(manifest) if !xcode => search::swift_targets(&manifest),
            _ => Vec::new(),
        }
    }
}
