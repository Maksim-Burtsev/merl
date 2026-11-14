use super::{App, Kind, Path, PathBuf, TypeVia, search};

impl App {
    pub(super) fn literal_element(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        n: &str,
        hops: usize,
    ) -> Option<TypeVia> {
        let value = search::ts_array_literal_element(text.lines().nth(line1 - 1)?, n)?;
        self.binding_type(
            kind,
            file,
            text,
            &search::Binding { line1, value },
            hops - 1,
        )
    }

    pub(in crate::app) fn named_element(
        &self,
        kind: Kind,
        at: &Path,
        written: &str,
    ) -> Option<(String, PathBuf)> {
        let list = self
            .type_decl(kind, at, written)
            .filter(|_| kind == Kind::Go)?;
        let text = self.text_of(&list.path)?;
        let decl = text
            .lines()
            .nth(list.line - 1)?
            .trim()
            .strip_prefix("type ")?;
        let holds = decl.trim_start().strip_prefix(list.name.as_str())?;
        Some((search::element_type(kind, holds)?, list.path))
    }
}
