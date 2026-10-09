use super::*;

enum Target {
    Variable(String),
    QualifiedVariable(String, String),
    Package(String),
    Method(String, String),
    Sub(String),
    MethodOnValue(String),
}

impl App {
    pub(super) fn perl_definition(&mut self, here: &Path) {
        self.offer_only = false;
        self.truncated.set(false);
        let line = self.line_str().to_owned();
        let Some(range) = search::perl_name(&line, self.col) else {
            self.message = "no word".into();
            return;
        };
        let name = line[range.clone()].to_owned();
        let text = self.buf.lines.join("\n");
        if line[..range.start].ends_with('{') && line[range.end..].starts_with('}') {
            self.message = format!("no definition for {name}");
            return;
        }
        let target = match search::perl_used_module(&line) == Some(range.clone()) {
            true => Target::Package(name.clone()),
            false => perl_target(&line, range, &name, self.col),
        };
        let (word, found) = match target {
            Target::Variable(word) => {
                let found = match search::perl_lexical(&text, self.line, self.col, &word) {
                    Some(at) => vec![Candidate {
                        hit: self.here_hit(here, at),
                        reason: Reason::Local,
                    }],
                    None => self.perl_by_name(here, &word, &search::perl_variable_patterns(&word)),
                };
                (word, found)
            }
            Target::QualifiedVariable(package, word) => {
                let patterns = search::perl_variable_patterns(&word);
                let found = self.perl_in_package(
                    here,
                    &package,
                    &word,
                    &patterns,
                    Reason::Path(package.clone()),
                );
                let found = match found.is_empty() {
                    true => self.perl_by_name(here, &word, &patterns),
                    false => found,
                };
                (word, found)
            }
            Target::Package(package) => {
                let found = self.perl_package(here, &package);
                (package, found)
            }
            Target::Method(package, word) => {
                let patterns = search::perl_sub_patterns(&word);
                let found = self.perl_in_package(
                    here,
                    &package,
                    &word,
                    &patterns,
                    Reason::Path(package.clone()),
                );
                let found = match found.is_empty() {
                    true => {
                        self.offer_only = !self.perl_files(here, &package).is_empty();
                        self.perl_by_name(here, &word, &patterns)
                    }
                    false => found,
                };
                (word, found)
            }
            Target::MethodOnValue(word) => {
                let found = self.perl_by_name(here, &word, &search::perl_sub_patterns(&word));
                (word, found)
            }
            Target::Sub(word) => {
                let patterns = search::perl_sub_patterns(&word);
                let imported = (search::perl_imports(&text).into_iter())
                    .filter(|(n, _)| *n == word)
                    .flat_map(|(_, module)| {
                        let reason = Reason::Import(module.clone());
                        self.perl_in_package(here, &module, &word, &patterns, reason)
                    })
                    .collect::<Vec<_>>();
                let found = match imported.is_empty() {
                    true => {
                        let mut all = patterns;
                        all.extend(search::perl_package_patterns(&word));
                        self.perl_by_name(here, &word, &all)
                    }
                    false => imported,
                };
                (word, found)
            }
        };
        self.show_definitions(Kind::Perl, &word, here, found, None);
    }

    fn here_hit(&self, here: &Path, line: usize) -> Hit {
        Hit {
            deleted: None,
            path: here.to_path_buf(),
            line1: line,
            byte_col: None,
            text: self.buf.lines[line - 1].clone(),
        }
    }

    fn perl_by_name(&mut self, here: &Path, word: &str, patterns: &[String]) -> Vec<Candidate> {
        (self.project_definitions(Kind::Perl, here, word, &patterns.join("|")))
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    fn perl_package(&mut self, here: &Path, package: &str) -> Vec<Candidate> {
        let found = self.perl_by_name(here, package, &search::perl_package_patterns(package));
        if !found.is_empty() {
            return found;
        }
        let shown = search::perl_module_file(package);
        let re = Regex::new(&search::perl_package_patterns(package).join("|"))
            .expect("an escaped name keeps it valid");
        (self.perl_files(here, package).into_iter())
            .map(|path| {
                let text = self.text_of(&path).unwrap_or_default();
                let lines: Vec<&str> = text.lines().collect();
                let line = lines.iter().position(|l| re.is_match(l)).unwrap_or(0);
                Candidate {
                    hit: Hit {
                        deleted: None,
                        text: lines.get(line).copied().unwrap_or_default().to_owned(),
                        path,
                        line1: line + 1,
                        byte_col: None,
                    },
                    reason: Reason::Module(shown.clone()),
                }
            })
            .collect()
    }

    fn perl_in_package(
        &mut self,
        here: &Path,
        package: &str,
        word: &str,
        patterns: &[String],
        reason: Reason,
    ) -> Vec<Candidate> {
        let re = Regex::new(&patterns.join("|")).expect("an escaped name keeps it valid");
        let mut found = Vec::new();
        for path in self.perl_files(here, package) {
            let Some(text) = self.text_of(&path) else {
                continue;
            };
            let lines: Vec<&str> = text.lines().collect();
            let literal = search::literal_lines(Kind::Perl, &text);
            for (i, l) in lines.iter().enumerate() {
                if literal.get(i) != Some(&true)
                    && re.is_match(l)
                    && search::perl_declares(&lines, i + 1, word, l)
                {
                    found.push(Candidate {
                        hit: Hit {
                            deleted: None,
                            path: path.clone(),
                            line1: i + 1,
                            byte_col: None,
                            text: (*l).to_owned(),
                        },
                        reason: reason.clone(),
                    });
                }
            }
        }
        found
    }

    fn perl_files(&mut self, here: &Path, package: &str) -> Vec<PathBuf> {
        let rel = search::perl_module_file(package);
        let pattern = search::perl_package_patterns(package).join("|");
        let mut files: Vec<PathBuf> = (self.project_grep(Kind::Perl, here, &pattern).into_iter())
            .map(|h| h.path)
            .chain(self.files.iter().filter(|f| f.ends_with(&rel)).cloned())
            .collect();
        files.dedup();
        let mut seen = HashSet::new();
        files.retain(|f| seen.insert(f.clone()));
        if !files.is_empty() {
            return files;
        }
        self.external_files(Kind::Perl);
        let roots = (self.external.get(&Kind::Perl))
            .map(|(roots, _)| roots.clone())
            .unwrap_or_default();
        search::perl_roots(&self.root, "")
            .into_iter()
            .chain(roots)
            .map(|r| r.join(&rel))
            .find(|f| f.is_file())
            .map(|f| {
                f.strip_prefix(&self.root)
                    .map_or(f.clone(), Path::to_path_buf)
            })
            .into_iter()
            .collect()
    }
}

fn perl_target(line: &str, range: std::ops::Range<usize>, name: &str, col: usize) -> Target {
    let before = &line[..range.start];
    let after = line[range.end..].trim_start();
    let sigil = before.ends_with(['$', '@', '%']) || before.ends_with("$#");
    let (package, last) = match name.rsplit_once("::") {
        Some((p, l)) => (Some(p.to_owned()), l.to_owned()),
        None => (None, name.to_owned()),
    };
    if sigil {
        return match package {
            Some(p) => Target::QualifiedVariable(p, last),
            None => Target::Variable(last),
        };
    }
    let declared =
        before.trim_start().starts_with("package ") || before.trim_start().starts_with("class ");
    if after.starts_with("->") || after.starts_with("::") || declared {
        return Target::Package(name.to_owned());
    }
    if let Some(package) = package {
        let on_last = col >= range.end - last.len();
        let capitalised = last.starts_with(|c: char| c.is_ascii_uppercase())
            && last.contains(|c: char| c.is_ascii_lowercase());
        let sub = !capitalised || after.starts_with('(');
        return match (sub, on_last) {
            (true, true) => Target::Method(package, last),
            (true, false) => Target::Package(package),
            (false, _) => Target::Package(name.to_owned()),
        };
    }
    let head = before.trim_end();
    match head.strip_suffix("->") {
        Some(receiver) => {
            let receiver = receiver.trim_end();
            let start = receiver
                .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == ':')
                .len();
            let class = &receiver[start..];
            let value = receiver[..start].ends_with(['$', '@', '%', '>', '}', ']', ')']);
            match class.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') && !value {
                true => Target::Method(class.to_owned(), last),
                false => Target::MethodOnValue(last),
            }
        }
        None => Target::Sub(last),
    }
}
