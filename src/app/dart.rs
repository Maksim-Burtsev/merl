//! `d` in a Dart file through its imports (#414): a prefix of `import … as p` and a name of
//! `import … show A` are looked for in the file the import names.

use super::*;

impl App {
    /// The declarations of `word` in the file the import binding the chain's first name, or the
    /// word itself, names: `get` of `http.get` behind `import 'package:http/http.dart' as http`,
    /// a top-level one there, or a member of the class `p.Cart.label` names. `None` when no
    /// `as` or `show` binds the name, the file is not on disk (no `pub get` yet), or it does not
    /// declare the word: a barrel's `export` is not followed, and the search by name takes over.
    pub(super) fn dart_imported(
        &self,
        here: &Path,
        text: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let (first, within) = match chain {
            [] => (word, None),
            [p] => (p.as_str(), None),
            [p, class] => (p.as_str(), Some(format!("{class}.{word}"))),
            _ => return None,
        };
        let (_, uri) = search::dart_imports(text)
            .into_iter()
            .find(|(name, _)| name == first)?;
        let sdk = search::dart_sdk();
        let file = search::dart_uri_file(&self.root, here, &uri, sdk.as_deref())?;
        let pattern = search::def_patterns(Kind::Dart, word).join("|");
        let hits = match file.is_absolute() {
            true => self.external_grep(Kind::Dart, std::slice::from_ref(&file), &pattern),
            false => self
                .grep(&pattern, false, false, |p| p == file)
                .unwrap_or_default(),
        };
        let found: Vec<Candidate> = self
            .declaring(Kind::Dart, word, hits)
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path)
                    .is_some_and(|t| search::qualified(Kind::Dart, &t, h.line, word) == within)
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::Import(uri.clone()),
            })
            .collect();
        (!found.is_empty()).then_some(found)
    }
}
