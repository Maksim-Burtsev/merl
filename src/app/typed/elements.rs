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
        let one = self.binding_type(
            kind,
            file,
            text,
            &search::Binding { line1, value },
            hops - 1,
        )?;
        let at = (file.to_path_buf(), line1);
        if self.reading.borrow().contains(&at) {
            return Some(one);
        }
        let outer = self.reading.replace(vec![at]);
        let kept = search::ts_array_pushes(text, line1, n).is_some_and(|pushed| {
            pushed.into_iter().all(|(line1, value)| {
                let b = search::Binding { line1, value };
                self.binding_type(kind, file, text, &b, hops - 1)
                    .is_some_and(|t| (&t.ty.path, t.ty.line) == (&one.ty.path, one.ty.line))
            })
        });
        self.reading.replace(outer);
        kept.then_some(one)
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
