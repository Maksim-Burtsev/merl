use super::*;

impl App {
    pub(super) fn proto_definitions(
        &mut self,
        text: &str,
        before: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r#"^\s*import\s+(?:(?:public|weak)\s+)?"([^"]*)""#).unwrap()
        });
        static PACKAGE: std::sync::LazyLock<Regex> =
            std::sync::LazyLock::new(|| Regex::new(r"(?m)^\s*package\s+([\w.]+)\s*;").unwrap());
        if let Some(path) = IMPORT
            .captures(self.line_str())
            .and_then(|c| c.get(1))
            .filter(|m| (m.start()..=m.end()).contains(&self.col))
        {
            let path = path.as_str().to_owned();
            return Some(self.proto_import(&path));
        }
        let qualifier = &before[before
            .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.')
            .len()..];
        let parts: Vec<&str> = qualifier.split('.').filter(|p| !p.is_empty()).collect();
        if !qualifier.ends_with('.') || parts.is_empty() {
            return None;
        }
        let own = PACKAGE
            .captures(text)
            .map_or(String::new(), |c| c[1].to_owned());
        let own: Vec<&str> = own.split('.').filter(|p| !p.is_empty()).collect();
        let outermost = if qualifier.starts_with('.') {
            0
        } else {
            own.len()
        };
        let join = |scope: &[&str], names: &[&str]| -> String {
            scope
                .iter()
                .chain(names)
                .copied()
                .collect::<Vec<_>>()
                .join(".")
        };
        for n in (0..=outermost).rev() {
            let scope = &own[..n];
            let base = join(scope, &parts[..1]);
            let pattern = format!(r"^\s*package\s+{}(?:\.[\w.]+)?\s*;", regex::escape(&base));
            let mut declared = self
                .grep(&pattern, false, false, |p| {
                    search::kind_of(p) == Some(Kind::Proto)
                })
                .unwrap_or_default();
            let outside = self.external_files(Kind::Proto);
            declared.extend(self.external_grep(Kind::Proto, &outside, &pattern));
            self.truncated.set(false);
            if declared.is_empty() {
                continue;
            }
            for k in (1..=parts.len()).rev() {
                let package = join(scope, &parts[..k]);
                let files: Vec<PathBuf> = declared
                    .iter()
                    .filter(|h| PACKAGE.captures(&h.text).is_some_and(|c| c[1] == package))
                    .map(|h| h.path.clone())
                    .collect();
                if files.is_empty() {
                    continue;
                }
                let nested = (k < parts.len()).then(|| format!("{}.{word}", parts[k..].join(".")));
                let pattern = search::def_patterns(Kind::Proto, word).join("|");
                let (outside, own): (Vec<PathBuf>, Vec<PathBuf>) =
                    files.into_iter().partition(|f| f.is_absolute());
                let mut hits = self
                    .grep(&pattern, false, false, |p| own.iter().any(|f| f == p))
                    .unwrap_or_default();
                hits.extend(self.external_grep(Kind::Proto, &outside, &pattern));
                hits.retain(|h| {
                    self.text_of(&h.path)
                        .and_then(|t| search::qualified(Kind::Proto, &t, h.line1, word))
                        == nested
                });
                let found: Vec<Candidate> = hits
                    .into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Path(package.clone()),
                    })
                    .collect();
                return (!found.is_empty()).then_some(found);
            }
            return None;
        }
        None
    }

    fn proto_import(&mut self, path: &str) -> Vec<Candidate> {
        let wanted = Path::new(path);
        let mut files: Vec<PathBuf> = self
            .files
            .iter()
            .filter(|f| f.ends_with(wanted))
            .cloned()
            .collect();
        if files.is_empty() {
            let outside = self.external_files(Kind::Proto);
            files.extend(outside.iter().find(|f| f.ends_with(wanted)).cloned());
        }
        files
            .into_iter()
            .map(|file| Candidate {
                reason: Reason::Module(
                    self.rel_to_its_root(Kind::Proto, &file)
                        .display()
                        .to_string(),
                ),
                hit: Hit {
                    text: self.text_of(&file).map_or_else(String::new, |t| {
                        t.lines().next().unwrap_or_default().to_owned()
                    }),
                    path: file,
                    line1: 1,
                    byte_col: None,
                    deleted: None,
                },
            })
            .collect()
    }
}
