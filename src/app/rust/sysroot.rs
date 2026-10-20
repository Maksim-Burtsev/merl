use super::{App, Candidate, Hit, Kind, Path, Reason, bound, search};

impl App {
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
            files.extend(self.sysroot_crate(krate)?.files);
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
        let found: Vec<Hit> = levels.iter().find_map(|level| {
            let at: Vec<&(Hit, bool, String)> =
                owned.iter().filter(|(_, _, t)| t == level).collect();
            let inherent: Vec<Hit> = at.iter().filter(|o| o.1).map(|o| o.0.clone()).collect();
            let found = if inherent.is_empty() {
                at.into_iter().map(|o| o.0.clone()).collect()
            } else {
                inherent
            };
            (!found.is_empty()).then_some(found)
        })?;
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
