use std::path::Path;

use super::{App, Candidate, Hit, Kind, Reason, search};

impl App {
    pub(super) fn scheme_renamed(&self, here: &Path, word: &str) -> Vec<Hit> {
        let pattern = format!(r"\([^\s()]+\s+{}\)", regex::escape(word));
        let mut hits = self.project_grep(Kind::Scheme, here, &pattern);
        hits.retain(|h| {
            let dir = h.path.parent().unwrap_or_else(|| Path::new(""));
            self.text_of(&h.path).is_some_and(|text| {
                search::scheme_renames(&text, h.line1, word, h.path == here, |f| {
                    dir.join(f) == here
                })
            })
        });
        hits
    }

    pub(super) fn definitions_grep(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        pattern: &str,
    ) -> Vec<Hit> {
        if !search::lisp(kind) {
            return self.project_grep(kind, here, pattern);
        }
        let alone = search::lisp_head_alone(kind);
        let mut hits = self.project_grep(kind, here, &format!("{pattern}|{alone}"));
        let Ok(alone) = regex::Regex::new(&alone) else {
            return hits;
        };
        hits.retain(|h| {
            !alone.is_match(&h.text)
                || self
                    .text_of(&h.path)
                    .is_some_and(|t| search::lisp_names_below(kind, &t, h.line1, word))
        });
        hits
    }

    pub(super) fn binding_lines(
        &self,
        kind: Kind,
        text: &str,
        line: usize,
        col: usize,
        bare: bool,
        name: &str,
    ) -> Vec<usize> {
        let found = if search::lisp(kind) && bare && line == self.line + 1 {
            search::lisp_bindings_at(kind, text, line, col, name)
        } else if let Some(l) = (kind == Kind::Julia && line == self.line + 1)
            .then(|| search::julia_comprehension_below(text, line, col, name))
            .flatten()
        {
            return vec![l];
        } else if kind == Kind::C && line == self.line + 1 {
            search::c_bindings(text, line, Some(col), name)
        } else if line == self.line + 1
            && self.line_str().get(..col).is_some_and(|lead| match kind {
                Kind::Lua => search::lua_binds_before(lead, name, false),
                Kind::Julia => search::julia_binds_before(lead, name),
                _ => false,
            })
        {
            return vec![line];
        } else if kind == Kind::Dart && line == self.line + 1 {
            search::dart_bindings_at(text, line, col, name)
        } else if kind == Kind::Jvm && line == self.line + 1 {
            search::jvm_bindings_at(text, line, col, name)
        } else if kind == Kind::Nix && line == self.line + 1 {
            search::nix_bindings_at(text, line, col, name)
        } else if kind == Kind::Solidity && line == self.line + 1 {
            search::bindings_at(kind, text, line, Some(col), name)
        } else {
            search::bindings(kind, text, line, name)
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
        let requires = unambiguous(&search::clojure_requires(text));
        let qualifier = |b: &str| {
            let at = b
                .rfind(|c: char| c.is_whitespace() || "()[]{}'`#@~^\"".contains(c))
                .map_or(0, |i| i + 1);
            let q = b.split_at(at).1;
            requires
                .iter()
                .find(|(n, _)| n == q)
                .map_or_else(|| q.to_owned(), |(_, ns)| ns.clone())
        };
        let target = after.strip_prefix('/').map(|a| {
            let end = a.find(|c: char| c.is_whitespace() || "()[]{}\"".contains(c));
            a.split_at(end.unwrap_or(a.len())).0
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
                if search::bindings(Kind::Clojure, text, self.line + 1, word).is_empty()
                    && !search::clojure_defines(text, word) =>
            {
                let ns = requires.iter().find(|(n, ns)| n == word && ns != word)?;
                (ns.1.clone(), word)
            }
            (None, None) => return None,
        };
        let files = search::clojure_ns_files(&ns, &self.files);
        let pattern = search::def_patterns(Kind::Clojure, name).join("|");
        let hits = if files.is_empty() {
            Vec::new()
        } else {
            self.declaring(Kind::Clojure, name, self.grep_in(&pattern, &files))
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

fn unambiguous(requires: &[(String, String)]) -> Vec<(String, String)> {
    let one = |(n, ns): &(String, String)| requires.iter().all(|(m, o)| m != n || o == ns);
    requires.iter().filter(|r| one(r)).cloned().collect()
}
