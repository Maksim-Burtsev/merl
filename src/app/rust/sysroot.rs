use super::{App, Candidate, Hit, Kind, Path, Reason, bound, rust_bare_type, rust_written_type, search};

impl App {
    pub(super) fn rust_std_written(&self, file: &Path, text: &str, name: &str) -> Option<String> {
        let lines: Vec<&str> = text.lines().collect();
        if name == "self" {
            return rust_bare_type(&search::rust_self_type(&lines, self.line)?.written);
        }
        if let Some(written) = rust_written_type(text, self.line, name) {
            return Some(written);
        }
        let bindings = search::bindings(Kind::Rust, text, self.line + 1, name);
        let [b] = bindings.as_slice() else {
            return None;
        };
        match search::rust_holds(&lines, b.line1 - 1, name) {
            Some(search::RustHolds::AssocCall {
                written_type,
                function,
            }) if matches!(function.as_str(), "new" | "from" | "with_capacity" | "default") => {
                rust_bare_type(&written_type)
            }
            Some(_) => None,
            None => self.rust_let_written(file, text, b.line1 - 1, name),
        }
    }

    pub(super) fn rust_std_call(
        &mut self,
        text: &str,
        name: &str,
        std: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        const DEREF: &[(&str, &[&str])] = &[
            ("String", &["String", "str"]),
            ("str", &["str"]),
            ("PathBuf", &["PathBuf", "Path"]),
            ("Path", &["Path"]),
            ("OsString", &["OsString", "OsStr"]),
            ("OsStr", &["OsStr"]),
            ("Vec", &["Vec"]),
            ("VecDeque", &["VecDeque"]),
            ("Option", &["Option"]),
            ("Result", &["Result"]),
            ("HashMap", &["HashMap"]),
            ("HashSet", &["HashSet"]),
            ("BTreeMap", &["BTreeMap"]),
            ("BTreeSet", &["BTreeSet"]),
        ];
        let (_, levels) = DEREF.iter().find(|(t, _)| *t == std)?;
        if bound(&search::imports(Kind::Rust, text), std).is_some_and(|p| {
            p.first()
                .is_none_or(|f| !matches!(f.as_str(), "std" | "core" | "alloc"))
        }) {
            return None;
        }
        let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
        let own = self
            .grep(&search::rust_type_decl_pattern(std), false, false, rs)
            .unwrap_or_default();
        if !own.is_empty() {
            return None;
        }
        let mut files = Vec::new();
        for krate in ["core", "alloc", "std"] {
            files.extend(self.sysroot_crate(krate).map(|c| c.files).unwrap_or_default());
        }
        let hits = search::grep_project(
            &self.root,
            &files,
            &search::rust_method_pattern(word),
            false,
            false,
            None,
            None,
        )
        .unwrap_or_default();
        let mut owned: Vec<(Hit, bool, String)> = Vec::new();
        for h in hits {
            let Some(t) = self.text_of(&h.path) else {
                continue;
            };
            let lines: Vec<&str> = t.lines().collect();
            if search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) == Some(&true) {
                continue;
            }
            let Some(m) = search::rust_method_at(&lines, h.line1) else {
                continue;
            };
            let inherent = match m.owner {
                search::RustOwner::Inherent if search::rust_stability(&lines, h.line1) => true,
                search::RustOwner::ImplOf(_) => false,
                _ => continue,
            };
            if let Some(ty) = search::rust_impl_type(lines[m.owner_line1 - 1]) {
                owned.push((h, inherent, ty));
            }
        }
        let found: Vec<Hit> = levels
            .iter()
            .find_map(|level| {
            let at: Vec<&(Hit, bool, String)> =
                owned.iter().filter(|(_, _, t)| t == level).collect();
            let inherent: Vec<Hit> = at.iter().filter(|o| o.1).map(|o| o.0.clone()).collect();
            let found = if inherent.is_empty() {
                at.into_iter().map(|o| o.0.clone()).collect()
            } else {
                inherent
            };
            (!found.is_empty()).then_some(found)
        })
            .unwrap_or_default();
        let reason = Reason::Receiver(format!("{name}: {std}"));
        Some(
            found
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
        )
    }
}
