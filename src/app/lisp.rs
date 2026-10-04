use super::*;

impl App {
    pub(super) fn clojure_qualified(
        &self,
        text: &str,
        before: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let requires = search::clojure_requires(text);
        let ns = match before.strip_suffix('/') {
            Some(b) => {
                let at = b
                    .rfind(|c: char| c.is_whitespace() || "()[]{}'`#@~^\"".contains(c))
                    .map_or(0, |i| i + 1);
                let q = &b[at..];
                requires
                    .iter()
                    .find(|(n, _)| n == q)
                    .map_or(q, |(_, ns)| ns)
            }
            None if search::bindings(Kind::Clojure, text, self.line + 1, word).is_empty() => {
                &requires.iter().find(|(n, ns)| n == word && ns != word)?.1
            }
            None => return None,
        };
        let files = search::clojure_ns_files(ns, &self.files);
        let pattern = search::def_patterns(Kind::Clojure, word).join("|");
        let hits = match files.is_empty() {
            true => Vec::new(),
            false => self.declaring(Kind::Clojure, word, self.grep_in(&pattern, &files)),
        };
        if hits.is_empty() && !files.is_empty() {
            return None;
        }
        let found = hits.into_iter().map(|hit| Candidate {
            reason: Reason::Import(hit.path.display().to_string()),
            hit,
        });
        Some(found.collect())
    }
}
