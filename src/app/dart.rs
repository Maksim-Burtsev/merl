use super::{App, Candidate, Kind, Path, Reason, search};

impl App {
    pub(super) fn word_here(&self, kind: Option<Kind>) -> Option<(std::ops::Range<usize>, &str)> {
        let line = self.line_str();
        let literal = || {
            search::literal_lines(Kind::Dart, &self.buf.lines.join("\n")).get(self.line)
                == Some(&true)
        };
        if kind == Some(Kind::Dart) && literal() {
            search::word_at(line, self.col, "")
        } else {
            search::definition_word(kind, line, self.col)
        }
    }

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
        if let [_, class] = chain {
            patterns.push(search::dart_bare_constructor(class, word));
        }
        let pattern = patterns.join("|");
        let built = chain.is_empty().then(|| format!("{word}.{word}"));
        let hits = if file.is_absolute() {
            self.external_grep(Kind::Dart, std::slice::from_ref(&file), &pattern)
        } else {
            self.grep(&pattern, false, false, |p| p == file)
                .unwrap_or_default()
        };
        let source = self.text_of(&file)?;
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
