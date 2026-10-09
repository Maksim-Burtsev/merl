use super::*;

impl App {
    pub(super) fn file_imported(
        &self,
        kind: Kind,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        match kind {
            Kind::Dart => self.dart_imported(here, text, before, chain, word),
            Kind::Haskell => self.haskell_imported(here, text, before, chain, word),
            Kind::Solidity => self.solidity_imported(here, text, before, chain, word),
            _ => None,
        }
    }

    fn haskell_imported(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let imports = search::haskell_imports(text);
        let named = |q: &str| -> Vec<String> {
            imports
                .iter()
                .filter(|i| i.alias.as_deref().unwrap_or(&i.module) == q)
                .map(|i| i.module.clone())
                .collect()
        };
        let qualifier = self.line_str()[before.len()..]
            .strip_prefix(word)
            .is_some_and(|after| after.starts_with('.'));
        if qualifier {
            let modules = named(&[chain, &[word.to_owned()]].concat().join("."));
            if !modules.is_empty() {
                let files = modules
                    .iter()
                    .flat_map(|m| self.import_module_files(here, m))
                    .collect();
                return Some(self.module_candidates(files));
            }
        }
        let modules: Vec<String> = match chain {
            [] if before.ends_with('.') && !before.ends_with("..") => return None,
            [] if !search::bindings(Kind::Haskell, text, self.line + 1, word).is_empty() => {
                return None;
            }
            [] => imports
                .iter()
                .filter(|i| !i.qualified && i.names.iter().any(|n| n == word))
                .map(|i| i.module.clone())
                .collect(),
            _ => named(&chain.join(".")),
        };
        if modules.is_empty() {
            return None;
        }
        let files: Vec<PathBuf> = modules
            .iter()
            .flat_map(|m| self.import_module_files(here, m))
            .collect();
        if files.is_empty() {
            return Some(Vec::new());
        }
        let pattern = search::def_patterns(Kind::Haskell, word).join("|");
        let hits = self
            .grep(&pattern, false, false, |p| files.iter().any(|f| f == p))
            .unwrap_or_default();
        let mut literal: HashMap<PathBuf, Vec<bool>> = HashMap::new();
        let found: Vec<Candidate> = self
            .declaring(Kind::Haskell, word, hits)
            .into_iter()
            .filter(|h| {
                let lines = literal.entry(h.path.clone()).or_insert_with(|| {
                    let text = self.text_of(&h.path).unwrap_or_default();
                    search::literal_lines(Kind::Haskell, &text)
                });
                lines.get(h.line1 - 1) != Some(&true)
            })
            .map(|hit| Candidate {
                reason: Reason::Import(hit.path.display().to_string()),
                hit,
            })
            .collect();
        (!found.is_empty()).then_some(found)
    }

    fn import_module_files(&self, here: &Path, module: &str) -> Vec<PathBuf> {
        let files = search::module_files(
            Kind::Haskell,
            &self.root,
            &self.files,
            here,
            &[module.to_owned()],
        );
        let package = |f: &Path| -> Option<PathBuf> {
            f.ancestors().skip(1).map(Path::to_path_buf).find(|dir| {
                self.files.iter().any(|p| {
                    p.parent() == Some(dir.as_path())
                        && (p.extension().is_some_and(|e| e == "cabal")
                            || p.ends_with("package.yaml"))
                })
            })
        };
        let own: Vec<PathBuf> = (files.iter())
            .filter(|f| package(f) == package(here))
            .cloned()
            .collect();
        match own.is_empty() {
            true => files,
            false => own,
        }
    }
}
