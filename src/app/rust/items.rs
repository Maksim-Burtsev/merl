use super::{App, Hit, Kind, Path, Typed, search};

impl App {
    pub(super) fn rust_members(&self, ty: &Typed, word: &str) -> Vec<Hit> {
        self.rust_items(ty, &search::rust_method_pattern(word))
    }

    pub(super) fn rust_items(&self, ty: &Typed, pattern: &str) -> Vec<Hit> {
        let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
        let decl = search::rust_type_decl_pattern(&ty.name);
        let files = self.rust_type_files(ty);
        let method = |hits: Vec<Hit>, owner: &dyn Fn(&search::RustOwner, &str) -> bool| {
            hits.into_iter()
                .filter(|h| {
                    self.text_of(&h.path).is_some_and(|t| {
                        let lines: Vec<&str> = t.lines().collect();
                        search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) != Some(&true)
                            && search::rust_method_at(&lines, h.line1)
                                .is_some_and(|m| owner(&m.owner, lines[m.owner_line1 - 1]))
                    })
                })
                .collect::<Vec<Hit>>()
        };
        let found = method(self.grep_in(pattern, &files), &|o, line| {
            matches!(
                o,
                search::RustOwner::Inherent | search::RustOwner::ImplOf(_)
            ) && search::rust_impl_type(line).is_some_and(|t| t == ty.name)
        });
        let text_of_ty = self.text_of(&ty.path).unwrap_or_default();
        let lines_of_ty: Vec<&str> = text_of_ty.lines().collect();
        let home = search::rust_inline_mod(&lines_of_ty, ty.line - 1);
        let declared_twice_here = self.grep_in(&decl, std::slice::from_ref(&ty.path)).len() > 1;
        let found: Vec<Hit> = found
            .into_iter()
            .filter(|h| {
                !declared_twice_here
                    || h.path != ty.path
                    || search::rust_inline_mod(&lines_of_ty, h.line1 - 1) == home
            })
            .collect();
        let inherent: Vec<Hit> = found
            .iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    let lines: Vec<&str> = t.lines().collect();
                    search::rust_method_at(&lines, h.line1)
                        .is_some_and(|m| matches!(m.owner, search::RustOwner::Inherent))
                })
            })
            .cloned()
            .collect();
        if !inherent.is_empty() {
            return inherent;
        }
        if !found.is_empty() {
            return found;
        }
        let impls = format!(
            r"^\s*(?:(?:unsafe|const)\s+)*impl\b[^{{]*\bfor\s+&?(?:\w+::)*{}\b",
            regex::escape(&ty.name)
        );
        let traits: Vec<String> = self
            .grep_in(&impls, &files)
            .iter()
            .filter_map(|h| search::rust_impl_trait(&h.text))
            .collect();
        if traits.is_empty() {
            return Vec::new();
        }
        let hits = self
            .grep(pattern, false, false, |p| rs(p))
            .unwrap_or_default();
        method(
            hits,
            &|o, _| matches!(o, search::RustOwner::Trait(t) if traits.contains(t)),
        )
        .into_iter()
        .filter(|h| !h.text.trim_end().ends_with(';'))
        .collect()
    }
}
