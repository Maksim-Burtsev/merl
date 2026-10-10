use super::*;

impl App {
    /// The word under the cursor as `d` reads it: [`search::definition_word`], save on a line
    /// inside a Dart `'''` string, where every `$` interpolates and none is part of a name (#414).
    pub(super) fn word_here(&self, kind: Option<Kind>) -> Option<(std::ops::Range<usize>, &str)> {
        let line = self.line_str();
        let literal = || {
            search::literal_lines(Kind::Dart, &self.buf.lines.join("\n")).get(self.line)
                == Some(&true)
        };
        match kind == Some(Kind::Dart) && literal() {
            true => search::word_at(line, self.col, ""),
            false => search::definition_word(kind, line, self.col),
        }
    }

    /// The declarations of `word` in the file an import of `text` names: `get` of `http.get`
    /// behind `import 'package:http/http.dart' as http`, a top-level one there; `weigh` behind
    /// `show weigh`; a member of the class the chain names, `p.Cart.label` or `Cart.label` behind
    /// `show Cart`. `before` is the line up to the word: a member hanging off a call, `f().x`,
    /// has no chain and is no imported name. `None` when no `as` or `show` binds the name, the
    /// file is not on disk (no `pub get` yet), or nothing in its code declares the word: a
    /// barrel's `export` is not followed, and the search by name takes over.
    pub(super) fn dart_imported(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let imports = search::dart_imports(text);
        let bound = |name: &str, prefix: bool| {
            imports
                .iter()
                .find(|i| i.name == name && i.is_prefix == prefix)
                .map(|i| i.uri_as_written.clone())
        };
        let (uri, within) = match chain {
            [] if before.trim_end().ends_with('.') => return None,
            [] => (bound(word, false)?, None),
            [p] => match bound(p, true) {
                Some(uri) => (uri, None),
                None => (bound(p, false)?, Some(format!("{p}.{word}"))),
            },
            [p, class] => (bound(p, true)?, Some(format!("{class}.{word}"))),
            _ => return None,
        };
        let sdk = search::dart_sdk();
        let file = search::dart_uri_file(&self.root, here, &uri, sdk.as_deref())?;
        let mut patterns = search::def_patterns(Kind::Dart, word);
        self.spelling_cut(Kind::Dart, word, &mut patterns);
        let pattern = patterns.join("|");
        // `Tariff(…)` behind `show Tariff` is built by the class's own constructor too.
        let built = chain.is_empty().then(|| format!("{word}.{word}"));
        let hits = match file.is_absolute() {
            true => self.external_grep(Kind::Dart, std::slice::from_ref(&file), &pattern),
            false => self
                .grep(&pattern, false, false, |p| p == file)
                .unwrap_or_default(),
        };
        let source = self.text_of(&file)?;
        // A line in a `/* */` or a `'''` declares nothing, and must not end the lookup here.
        let literal = search::literal_lines(Kind::Dart, &source);
        let found: Vec<Candidate> = self
            .declaring(Kind::Dart, word, hits)
            .into_iter()
            .filter(|h| {
                literal.get(h.line1 - 1) != Some(&true)
                    && [&within, &built].contains(&&search::qualified(
                        Kind::Dart,
                        &source,
                        h.line1,
                        word,
                    ))
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::Import(uri.clone()),
            })
            .collect();
        (!found.is_empty()).then_some(found)
    }
}
