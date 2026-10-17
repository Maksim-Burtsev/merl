use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, search};

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
        let qualifier = self
            .line_str()
            .get(before.len()..)
            .and_then(|r| r.strip_prefix(word))
            .is_some_and(|r| r.starts_with('.') && !r.starts_with(".."));
        if chain.is_empty()
            && qualifier
            && !before.trim_end().ends_with('.')
            && search::bindings(Kind::Dart, text, self.line + 1, word).is_empty()
            && let Some(d) = search::dart_directives(text)
                .into_iter()
                .flatten()
                .find(|d| d.keyword == "import" && d.prefix.as_deref() == Some(word))
        {
            let hit = Hit {
                deleted: None,
                path: here.to_path_buf(),
                line1: d.line1,
                byte_col: None,
                text: self.buf.lines.get(d.line1 - 1)?.clone(),
            };
            let reason = Reason::Import(d.uri);
            return Some(vec![Candidate { hit, reason }]);
        }
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

    pub(super) fn dart_visible(
        &self,
        here: &Path,
        word: &str,
        tail: &str,
        found: &mut Vec<Candidate>,
    ) {
        let Some(files) = self.dart_visible_files(here, word) else {
            return;
        };
        let top = |c: &Candidate| !c.hit.text.starts_with(char::is_whitespace);
        if !found.iter().any(|c| top(c) && files.contains(&c.hit.path)) {
            return;
        }
        let Some(text) = self.text_of(here) else {
            return;
        };
        let body = search::dart_body_at(&text, self.line);
        let built = format!("{word}.{word}");
        found.retain(|c| {
            let constructor = || {
                self.text_of(&c.hit.path).is_some_and(|t| {
                    search::qualified(Kind::Dart, &t, c.hit.line1, word).as_ref() == Some(&built)
                })
            };
            if c.hit.path.is_absolute() {
                true
            } else if top(c) {
                files.contains(&c.hit.path)
            } else if constructor() {
                search::dart_called(tail) && files.contains(&c.hit.path)
            } else {
                body.as_ref().is_none_or(|(lines, supers)| {
                    *supers || (c.hit.path == here && lines.contains(&c.hit.line1))
                })
            }
        });
    }

    fn dart_library(
        &self,
        file: PathBuf,
        sdk: Option<&Path>,
        owned: bool,
    ) -> Option<(Vec<PathBuf>, Vec<search::DartDirective>)> {
        let directives = search::dart_directives(&self.text_of(&file)?)?;
        if let Some(d) = directives.iter().find(|d| d.keyword == "part of") {
            let owner = search::dart_uri_file(&self.root, &file, &d.uri, sdk)?;
            return if owned {
                None
            } else {
                self.dart_library(owner, sdk, true)
            };
        }
        let mut files: Vec<PathBuf> = directives
            .iter()
            .filter(|d| d.keyword == "part")
            .filter_map(|d| search::dart_uri_file(&self.root, &file, &d.uri, sdk))
            .collect();
        files.insert(0, file);
        Some((files, directives))
    }

    fn dart_visible_files(&self, here: &Path, word: &str) -> Option<Vec<PathBuf>> {
        let sdk = search::dart_sdk();
        let sdk = sdk.as_deref();
        let (mut files, directives) = self.dart_library(here.to_path_buf(), sdk, false)?;
        let mut queue: Vec<(PathBuf, search::DartDirective)> = directives
            .into_iter()
            .filter(|d| d.keyword == "import" && d.prefix.is_none())
            .map(|d| (files[0].clone(), d))
            .collect();
        while let Some((from, d)) = queue.pop() {
            if d.conditional {
                return None;
            }
            if d.uri.starts_with("dart:")
                || d.show
                    .as_ref()
                    .is_some_and(|s| !s.iter().any(|n| n == word))
                || d.hide.iter().any(|n| n == word)
            {
                continue;
            }
            let file = search::dart_uri_file(&self.root, &from, &d.uri, sdk)?;
            if files.contains(&file) {
                continue;
            }
            if file.is_absolute() {
                files.push(file);
                continue;
            }
            let (library, directives) = self.dart_library(file, sdk, false)?;
            queue.extend(
                directives
                    .into_iter()
                    .filter(|d| d.keyword == "export")
                    .map(|d| (library[0].clone(), d)),
            );
            files.extend(library);
        }
        Some(files)
    }
}
