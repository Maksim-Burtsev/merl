use super::{App, Hit, Kind, Typed, search};

impl App {
    pub(super) fn ts_augmentations(&self, ty: &Typed, word: &str) -> Vec<Hit> {
        let pattern = search::def_patterns(Kind::TsJs, word).join("|");
        let cut = self.truncated.get();
        let hits = self.project_definitions(Kind::TsJs, &ty.path, word, &pattern);
        self.truncated.set(cut);
        hits.into_iter()
            .filter(|h| {
                (self.text_of(&h.path))
                    .and_then(|t| search::ts_augmented_module(&t, h.line1, &ty.name))
                    .is_some_and(|spec| {
                        search::module_files(Kind::TsJs, &self.root, &self.files, &h.path, &[spec])
                            .contains(&ty.path)
                    })
            })
            .collect()
    }

    pub(super) fn ts_augmented_class(&self, decl: &Hit, parts: &[String]) -> Option<Hit> {
        let text = self.text_of(&decl.path)?;
        let spec = search::ts_augmented_module(&text, decl.line1, parts.last()?)?;
        let [file] = search::module_files(Kind::TsJs, &self.root, &self.files, &decl.path, &[spec])
            .try_into()
            .ok()?;
        self.declaration(Kind::TsJs, &file, parts)
            .filter(|d| d.path == file)
    }
}
