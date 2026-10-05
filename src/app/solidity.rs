use super::*;

impl App {
    pub(super) fn solidity_imported(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let imports = search::solidity_imports(text);
        let (import, name) = match chain {
            [] if before.trim_end().ends_with('.') => return None,
            [] => imports
                .iter()
                .find_map(|i| Some((i, i.name.clone().filter(|_| i.bound == word)?)))?,
            [q, ..] => (
                imports.iter().find(|i| i.bound == *q && i.name.is_none())?,
                word.to_owned(),
            ),
        };
        let file = search::solidity_file(&self.root, &self.files, here, &import.path)?;
        let pattern = search::def_patterns(Kind::Solidity, &name).join("|");
        let hits = match self.files.contains(&file) {
            false => self.external_grep(Kind::Solidity, std::slice::from_ref(&file), &pattern),
            true => self
                .grep(&pattern, false, false, |p| p == file)
                .unwrap_or_default(),
        };
        let source = self.text_of(&file)?;
        let literal = search::literal_lines(Kind::Solidity, &source);
        let mut hits = self.declaring(Kind::Solidity, &name, hits);
        hits.retain(|h| literal.get(h.line - 1) != Some(&true));
        let within = (chain.len() > 1).then(|| format!("{}.{name}", chain[1..].join(".")));
        if hits
            .iter()
            .any(|h| search::qualified(Kind::Solidity, &source, h.line, &name) == within)
        {
            hits.retain(|h| search::qualified(Kind::Solidity, &source, h.line, &name) == within);
        }
        let found: Vec<Candidate> = hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Import(import.path.clone()),
            })
            .collect();
        (!found.is_empty()).then_some(found)
    }
}
