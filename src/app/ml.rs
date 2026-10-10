use super::*;

fn capitalized(name: &str) -> String {
    let mut c = name.chars();
    c.next()
        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}

fn module_of_file(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(capitalized)
        .unwrap_or_default()
}

impl App {
    pub(super) fn ml_definitions(
        &mut self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        word: &str,
        dotted: bool,
    ) -> Option<Vec<Candidate>> {
        if !matches!(kind, Kind::Ocaml | Kind::Fsharp) {
            return None;
        }
        let at = |path: &Path, line: usize, text: &str| Hit {
            deleted: None,
            path: path.to_path_buf(),
            line1: line,
            byte_col: None,
            text: text.to_owned(),
        };
        let module = chain
            .last()
            .filter(|m| m.starts_with(|c: char| c.is_ascii_uppercase()));
        let label = self.word_here(Some(kind)).is_some_and(|(r, _)| {
            let line = self.line_str();
            let before = line[..r.start].trim_end();
            let after = line[r.end..].trim_start();
            (before.ends_with(['{', ';']) || before.ends_with(" with"))
                && after.starts_with('=')
                && !after.starts_with("==")
        });
        if module.is_none()
            && !dotted
            && !label
            && let Some(line) = search::ml_local(kind, &self.buf.lines, self.line + 1, word)
        {
            let hit = at(here, line, &self.buf.lines[line - 1]);
            return Some(vec![Candidate {
                hit,
                reason: Reason::Local,
            }]);
        }
        let pattern = search::def_patterns(kind, word).join("|");
        if pattern.is_empty() {
            return Some(Vec::new());
        }
        let mut hits = self.project_definitions(kind, here, word, &pattern);
        let mut texts: HashMap<PathBuf, Vec<String>> = HashMap::new();
        let mut lines_of = |a: &Self, p: &Path| -> Vec<String> {
            texts
                .entry(p.to_path_buf())
                .or_insert_with(|| {
                    a.text_of(p)
                        .map_or_else(Vec::new, |t| t.lines().map(str::to_owned).collect())
                })
                .clone()
        };
        hits.retain(|h| {
            let lines = lines_of(self, &h.path);
            search::literal_lines(kind, &lines.join("\n")).get(h.line1 - 1) != Some(&true)
        });
        if label {
            let field = Regex::new(&format!(r"(?:^|[{{;\s]){}\s*:", regex::escape(word)))
                .expect("an escaped name keeps the pattern valid");
            hits.retain(|h| field.is_match(&h.text));
        }
        let implemented: Vec<PathBuf> = hits
            .iter()
            .filter(|h| !search::ml_signature(&h.path))
            .map(|h| h.path.clone())
            .collect();
        hits.retain(|h| {
            search::ml_implementation(&h.path).is_none_or(|ml| !implemented.contains(&ml))
        });
        let by_name = |hits: Vec<Hit>, reason: Reason| -> Vec<Candidate> {
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect()
        };
        if let Some(m) = module {
            let named: Vec<Hit> = hits
                .iter()
                .filter(|h| {
                    let t = h.text.trim_start();
                    (t.starts_with("module ") || t.starts_with("namespace "))
                        && t.contains(&format!("{m}.{word}"))
                        || search::ml_modules(&h.path, &lines_of(self, &h.path), h.line1)
                            .contains(m)
                })
                .cloned()
                .collect();
            let defined = |h: &Hit| !h.text.trim_start().starts_with("val ");
            let named = match kind == Kind::Ocaml && named.iter().any(defined) {
                true => named.into_iter().filter(defined).collect(),
                false => named,
            };
            if !named.is_empty() {
                return Some(by_name(named, Reason::Path(m.clone())));
            }
            let declared = self
                .files
                .iter()
                .any(|f| search::kind_of(f) == Some(kind) && module_of_file(f) == *m)
                || !self
                    .project_definitions(kind, here, m, &search::def_patterns(kind, m).join("|"))
                    .is_empty();
            if declared {
                self.offer_only = true;
                return Some(by_name(hits, Reason::ByName));
            }
            let files: Vec<PathBuf> = (self.external_files(kind).iter())
                .filter(|f| module_of_file(f) == *m)
                .cloned()
                .collect();
            let found = self.external_grep(kind, &files, &pattern);
            return Some(by_name(found, Reason::Path(m.clone())));
        }
        if !dotted && !label {
            let line = self.line + 1;
            hits.retain(|h| {
                h.path != here || h.line1 <= line || h.text.trim_start().starts_with("and ")
            });
            let mine = search::ml_implementation(here).unwrap_or_else(|| here.to_path_buf());
            let own: Vec<Hit> = hits
                .iter()
                .filter(|h| {
                    (h.path == here || h.path == mine) && !(h.path == here && h.line1 == line)
                })
                .cloned()
                .collect();
            if !own.is_empty() {
                let reason = match mine == here {
                    true => Reason::File,
                    false => Reason::Path(module_of_file(&mine)),
                };
                return Some(by_name(own, reason));
            }
        }
        Some(by_name(hits, Reason::ByName))
    }
}
