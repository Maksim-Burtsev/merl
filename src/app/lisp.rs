use super::*;

impl App {
    pub(super) fn binding_lines(
        &self,
        kind: Kind,
        text: &str,
        line: usize,
        col: usize,
        bare: bool,
        name: &str,
    ) -> Vec<usize> {
        let found = match search::lisp(kind) && bare && line == self.line + 1 {
            true => search::lisp_bindings_at(kind, text, line, col, name),
            false => search::bindings(kind, text, line, name),
        };
        found.iter().map(|b| b.line1).collect()
    }

    pub(super) fn clojure_qualified(
        &self,
        text: &str,
        before: &str,
        word: &str,
        after: &str,
    ) -> Option<Vec<Candidate>> {
        let requires = search::clojure_requires(text);
        let qualifier = |b: &str| {
            let at = b
                .rfind(|c: char| c.is_whitespace() || "()[]{}'`#@~^\"".contains(c))
                .map_or(0, |i| i + 1);
            let q = &b[at..];
            requires
                .iter()
                .find(|(n, _)| n == q)
                .map_or(q.to_owned(), |(_, ns)| ns.clone())
        };
        let target = after.strip_prefix('/').map(|a| {
            let end = a.find(|c: char| c.is_whitespace() || "()[]{}\"".contains(c));
            &a[..end.unwrap_or(a.len())]
        });
        let (ns, name) = match (target.filter(|t| !t.is_empty()), before.strip_suffix('/')) {
            (Some(name), _) => {
                let ns = qualifier(&format!("{before}{word}"));
                let alias = requires.iter().any(|(n, ns)| n == word && ns != word);
                if !alias && search::clojure_ns_files(&ns, &self.files).is_empty() {
                    return None;
                }
                (ns, name)
            }
            (None, Some(b)) => (qualifier(b), word),
            (None, None)
                if search::bindings(Kind::Clojure, text, self.line + 1, word).is_empty() =>
            {
                let ns = requires.iter().find(|(n, ns)| n == word && ns != word)?;
                (ns.1.clone(), word)
            }
            (None, None) => return None,
        };
        let files = search::clojure_ns_files(&ns, &self.files);
        let pattern = search::def_patterns(Kind::Clojure, name).join("|");
        let hits = match files.is_empty() {
            true => Vec::new(),
            false => self.declaring(Kind::Clojure, name, self.grep_in(&pattern, &files)),
        };
        if hits.is_empty() && !files.is_empty() {
            return (name != word).then(|| self.module_candidates(files));
        }
        let found = hits.into_iter().map(|hit| Candidate {
            reason: Reason::Import(hit.path.display().to_string()),
            hit,
        });
        Some(found.collect())
    }
}
