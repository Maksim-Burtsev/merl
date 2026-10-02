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
        if chain.is_empty() && qualifier {
            let modules = named(word);
            if !modules.is_empty() {
                let files = modules
                    .iter()
                    .flat_map(|m| self.import_module_files(here, m))
                    .collect();
                return Some(self.module_candidates(files));
            }
        }
        let modules: Vec<String> = match chain {
            [] if before.trim_end().ends_with('.') => return None,
            [] if !search::bindings(Kind::Haskell, text, self.line + 1, word).is_empty() => {
                return None;
            }
            [] => imports
                .iter()
                .filter(|i| i.names.iter().any(|n| n == word))
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
        let found: Vec<Candidate> = self
            .declaring(Kind::Haskell, word, hits)
            .into_iter()
            .map(|hit| Candidate {
                reason: Reason::Import(hit.path.display().to_string()),
                hit,
            })
            .collect();
        (!found.is_empty()).then_some(found)
    }

    fn import_module_files(&self, here: &Path, module: &str) -> Vec<PathBuf> {
        search::module_files(
            Kind::Haskell,
            &self.root,
            &self.files,
            here,
            &[module.to_owned()],
        )
    }
}
