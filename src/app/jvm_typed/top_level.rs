use super::{JvmType, WrittenType};
use crate::app::{App, Hit, Kind, Path, search};

impl App {
    pub(super) fn kotlin_top_level_returned(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        ahead: &str,
        name: &str,
    ) -> Option<WrittenType> {
        let hit = self.kotlin_top_level_fun(file, text, line1, ahead, name)?;
        let written = search::kotlin_top_level_return(&hit.text, name)?;
        let t = self.text_of(&hit.path)?;
        Some(WrittenType {
            resolved: self.jvm_type_at(&hit.path, &t, &written),
            written,
        })
    }

    pub(super) fn kotlin_top_level_component(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        index: usize,
    ) -> Option<WrittenType> {
        let ahead = text.lines().nth(line1.checked_sub(1)?)?;
        let hit = self.kotlin_top_level_fun(file, text, line1, ahead, name)?;
        let t = self.text_of(&hit.path)?;
        if let Some((pair, items)) = search::kotlin_top_level_pair(&hit.text, name) {
            matches!(self.jvm_type_at(&hit.path, &t, &pair), JvmType::Outside(_)).then_some(())?;
            let written = items.get(index)?.clone();
            return Some(WrittenType {
                resolved: self.jvm_type_at(&hit.path, &t, &written),
                written,
            });
        }
        let JvmType::Project(decl) = self
            .kotlin_top_level_returned(file, text, line1, ahead, name)?
            .resolved
        else {
            return None;
        };
        let t = self.text_of(&decl.path)?;
        let written = search::kotlin_component_type(&t, decl.line1, index)?;
        Some(WrittenType {
            resolved: self.jvm_type_at(&decl.path, &t, &written),
            written,
        })
    }

    fn kotlin_top_level_fun(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        ahead: &str,
        name: &str,
    ) -> Option<Hit> {
        (file.extension().is_some_and(|e| e == "kt")
            && search::jvm_enclosing_types(text, line1).is_empty()
            && search::kotlin_top_level_body(text, line1, ahead)
            && search::path_bindings(Kind::Jvm, file, text, line1, name).is_empty())
        .then_some(())?;
        let pattern = search::def_patterns(Kind::Jvm, name).join("|");
        let cut = self.truncated.get();
        let mut hits = self.project_definitions(Kind::Jvm, file, name, &pattern);
        let full = self.truncated.get() && !cut;
        self.truncated.set(cut);
        hits.retain(|h| {
            h.path.extension().is_some_and(|e| e == "kt")
                && self.jvm_code(h)
                && self
                    .text_of(&h.path)
                    .is_some_and(|t| search::jvm_enclosing_types(&t, h.line1).is_empty())
        });
        let ([hit], false) = (&hits[..], full) else {
            return None;
        };
        let t = self.text_of(&hit.path)?;
        let package = search::jvm_package(&t);
        let base = |p: &[String]| p.split_last().map(|(_, b)| b.join("."));
        let imports = search::jvm_imports_at(file, text, line1);
        let seen = match imports.iter().find(|i| i.bound_name == name) {
            Some(i) => i.path.last().is_some_and(|l| l == name) && base(&i.path) == package,
            None => {
                package == search::jvm_package(text)
                    || imports
                        .iter()
                        .any(|i| i.bound_name == "*" && base(&i.path) == package)
            }
        };
        seen.then(|| hit.clone())
    }
}
