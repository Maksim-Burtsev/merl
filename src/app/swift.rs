use super::*;

impl App {
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
        // A cut in a grep whose result is dropped says nothing about the list shown in the end.
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
                        .and_then(|t| search::qualified(Kind::Swift, &t, h.line, word))
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
                    return None;
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
                return None;
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
            let place = search::swift_type_decl(&h.text)
                .then(|| self.text_of(&h.path))
                .flatten()
                .and_then(|t| search::swift_type_place(&t, h.line));
            match place {
                Some(search::SwiftTypePlace::Function(f)) => {
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
                .and_then(|t| search::swift_type_place(&t, h.line));
            match place {
                Some(search::SwiftTypePlace::Nested(outer)) => {
                    search::swift_sees_nested(&self.buf.lines, &literal, self.line + 1, &outer)
                }
                _ => true,
            }
        });
    }
}
