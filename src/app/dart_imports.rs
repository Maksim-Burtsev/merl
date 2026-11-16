use super::dart::dart_names;
use super::{App, Candidate, Kind, Path, PathBuf, Reason, search};

impl App {
    fn dart_outside_by_import(&self, here: &Path, word: &str, found: &mut Vec<Candidate>) {
        let Some(first) = found.first() else {
            return;
        };
        let sdk: Vec<&PathBuf> = self
            .external
            .get(&Kind::Dart)
            .map(|(roots, _)| {
                roots
                    .iter()
                    .filter(|r| r.join("core/core.dart").is_file())
                    .collect()
            })
            .unwrap_or_default();
        if found.iter().all(|c| c.hit.path.is_absolute())
            && found.iter().any(|c| c.hit.path != first.hit.path)
            && !found
                .iter()
                .any(|c| sdk.iter().any(|s| c.hit.path.starts_with(s)))
            && self.dart_visible_files(here, word, None).is_some()
        {
            let imported = self.dart_from_imports(here, word);
            if !imported.is_empty() {
                *found = imported;
            }
        }
    }

    pub(super) fn dart_from_imports(&self, here: &Path, word: &str) -> Vec<Candidate> {
        let inherits = self
            .text_of(here)
            .is_none_or(|t| !matches!(search::dart_body_at(&t, self.line), Some((_, false))));
        let sdk = self.dart_sdk();
        let Some((library, directives)) = self
            .dart_library(here.to_path_buf(), sdk.as_deref(), false)
            .filter(|_| !inherits)
        else {
            return Vec::new();
        };
        let wanted = [None, Some(format!("{word}.{word}"))];
        let mut seen = std::collections::HashSet::new();
        directives
            .into_iter()
            .filter(|d| {
                d.keyword == "import"
                    && d.prefix.is_none()
                    && d.uri.starts_with("package:")
                    && dart_names(d, word)
            })
            .filter_map(|d| search::dart_uri_file(&self.root, &library[0], &d.uri, sdk.as_deref()))
            .filter(|file| file.is_absolute())
            .flat_map(|file| self.dart_exported(here, file, word, &wanted, &Reason::ByName, 0))
            .filter(|c| seen.insert((c.hit.path.clone(), c.hit.line1)))
            .collect()
    }

    pub(super) fn dart_outside_kept(
        &self,
        here: &Path,
        word: &str,
        before: &str,
        tail: &str,
        found: &mut Vec<Candidate>,
    ) {
        self.dart_unimported(here, word, found);
        self.dart_outside_constructor(word, tail, found);
        if before.trim_end().ends_with('@') && matches!(word, "override" | "deprecated") {
            found.retain(|c| !c.hit.text.starts_with(char::is_whitespace));
        }
    }

    fn dart_unimported(&self, here: &Path, word: &str, found: &mut Vec<Candidate>) {
        self.dart_outside_by_import(here, word, found);
        let Some((roots, _)) = self.external.get(&Kind::Dart) else {
            return;
        };
        let built = Some(format!("{word}.{word}"));
        let named = |c: &Candidate| {
            c.hit.path.is_absolute()
                && self.text_of(&c.hit.path).is_some_and(|t| {
                    let q = search::qualified(Kind::Dart, &t, c.hit.line1, word);
                    (q.is_none() && !c.hit.text.starts_with(char::is_whitespace)) || q == built
                })
        };
        let homes: Vec<Option<PathBuf>> = found
            .iter()
            .map(|c| search::dart_root_of(roots, &c.hit.path).filter(|_| named(c)))
            .collect();
        let mut wanted: Vec<PathBuf> = homes.iter().flatten().cloned().collect();
        wanted.sort();
        wanted.dedup();
        let Some(reached) = self.dart_import_roots(here, roots, &wanted) else {
            return;
        };
        let unreached: Vec<bool> = homes
            .iter()
            .map(|h| h.as_ref().is_some_and(|r| !reached.contains(r)))
            .collect();
        if unreached.iter().all(|u| *u) {
            return;
        }
        let mut drop = unreached.into_iter();
        found.retain(|_| !drop.next().unwrap_or(false));
    }

    fn dart_import_roots(
        &self,
        here: &Path,
        roots: &[PathBuf],
        wanted: &[PathBuf],
    ) -> Option<Vec<PathBuf>> {
        let sdk = self.dart_sdk();
        let sdk = sdk.as_deref();
        let mut reached: Vec<PathBuf> = roots
            .iter()
            .filter(|r| r.join("core/core.dart").is_file())
            .cloned()
            .collect();
        if wanted.iter().all(|w| reached.contains(w)) {
            return None;
        }
        let (library, directives) = self.dart_library(here.to_path_buf(), sdk, false)?;
        let mut queue: Vec<(PathBuf, search::DartDirective)> = directives
            .into_iter()
            .filter(|d| d.keyword == "import")
            .map(|d| (library[0].clone(), d))
            .collect();
        let mut seen = std::collections::HashSet::new();
        while let Some((from, d)) = queue.pop() {
            for uri in std::iter::once(&d.uri).chain(&d.alternatives) {
                if uri.starts_with("dart:") && uri != "dart:ui" {
                    continue;
                }
                let base = if uri.starts_with("package:") {
                    here
                } else {
                    &from
                };
                let file = search::dart_uri_file(&self.root, base, uri, sdk)?;
                if !seen.insert(file.clone()) {
                    continue;
                }
                if let Some(r) = search::dart_root_of(roots, &file)
                    && !reached.contains(&r)
                {
                    reached.push(r);
                    if wanted.iter().all(|w| reached.contains(w)) {
                        return None;
                    }
                }
                let (library, directives) = self.dart_library(file, sdk, false)?;
                queue.extend(
                    directives
                        .into_iter()
                        .filter(|d| d.keyword == "export")
                        .map(|d| (library[0].clone(), d)),
                );
            }
        }
        Some(reached)
    }
}
