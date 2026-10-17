use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, Regex, search};

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
        if let Some(found) = self.dart_member(here, text, before, chain, word) {
            return Some(found);
        }
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
        let bare = match chain {
            [_, class] => Some(search::dart_bare_constructor(class, word)),
            _ => None,
        };
        let built = chain.is_empty().then(|| format!("{word}.{word}"));
        let found =
            self.dart_declared_in(&file, word, bare, &[within, built], &Reason::Import(uri));
        (!found.is_empty()).then_some(found)
    }

    fn dart_declared_in(
        &self,
        file: &Path,
        word: &str,
        bare: Option<String>,
        wanted: &[Option<String>],
        reason: &Reason,
    ) -> Vec<Candidate> {
        let mut patterns = search::def_patterns(Kind::Dart, word);
        self.spelling_cut(Kind::Dart, word, &mut patterns);
        patterns.extend(bare);
        let pattern = patterns.join("|");
        let hits = if file.is_absolute() {
            self.external_grep(Kind::Dart, &[file.to_path_buf()], &pattern)
        } else {
            self.grep(&pattern, false, false, |p| p == file)
                .unwrap_or_default()
        };
        let Some(source) = self.text_of(file) else {
            return Vec::new();
        };
        let literal = search::literal_lines(Kind::Dart, &source);
        self.declaring(Kind::Dart, word, hits)
            .into_iter()
            .filter(|h| {
                literal.get(h.line1 - 1) != Some(&true)
                    && wanted.contains(&search::qualified(Kind::Dart, &source, h.line1, word))
            })
            .map(|hit| Candidate {
                hit,
                reason: reason.clone(),
            })
            .collect()
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

    fn dart_member(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let lines: Vec<&str> = text.lines().collect();
        let bound = |name: &str| {
            let bindings = search::bindings(Kind::Dart, text, self.line + 1, name);
            let [b] = bindings.as_slice() else {
                return None;
            };
            search::dart_typed(lines.get(b.line1 - 1)?, name)
        };
        let ty = match chain {
            [] if before.ends_with("..") => match search::dart_cascade(before)? {
                Ok(ty) => ty,
                Err(name) => bound(&name)?,
            },
            [r] if r == "this" || r == "super" => {
                search::dart_this_super(text, self.line, r == "super")?
            }
            [r] => bound(r)?,
            _ => return None,
        };
        let file = self.dart_class_file(here, &ty)?;
        let class = &ty.class;
        let made = |c: &str| {
            let bare = Some(search::dart_bare_constructor(class, c));
            let within = [Some(format!("{class}.{c}"))];
            let reason = Reason::Receiver(class.clone());
            self.dart_declared_in(&file, c, bare, &within, &reason)
        };
        if let Some(c) = &ty.constructor
            && !made(c)
                .iter()
                .any(|h| h.hit.text.contains(&format!("{class}.{c}")))
        {
            return None;
        }
        let found = made(word);
        (!found.is_empty()).then_some(found)
    }

    fn dart_class_file(&self, here: &Path, ty: &search::DartType) -> Option<PathBuf> {
        let sdk = search::dart_sdk();
        let sdk = sdk.as_deref();
        let files = match &ty.prefix {
            Some(p) => {
                let (library, directives) = self.dart_library(here.to_path_buf(), sdk, false)?;
                let d = directives
                    .iter()
                    .find(|d| d.keyword == "import" && d.prefix.as_ref() == Some(p))?;
                let file = search::dart_uri_file(&self.root, &library[0], &d.uri, sdk)?;
                self.dart_library(file, sdk, false)?.0
            }
            None => self.dart_visible_files(here, &ty.class)?,
        };
        let class = Regex::new(&search::def_patterns(Kind::Dart, &ty.class)[0]).ok()?;
        let mut declaring = files.into_iter().filter(|f| {
            !f.is_absolute()
                && self.text_of(f).is_some_and(|t| {
                    let literal = search::literal_lines(Kind::Dart, &t);
                    t.lines().enumerate().any(|(i, l)| {
                        !literal[i] && !l.starts_with(char::is_whitespace) && class.is_match(l)
                    })
                })
        });
        let file = declaring.next()?;
        declaring.next().is_none().then_some(file)
    }
}
