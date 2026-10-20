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
                return Some(found);
            }
            let next = match (keyword.as_str(), base) {
                ("class", Some(next)) if seen.insert(next.clone()) => next,
                _ => {
                    self.truncated.set(cut);
                    return self.swift_self_default(here, &own, word);
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
                return self.swift_self_default(here, &own, word);
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
