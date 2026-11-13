use super::{JvmType, WrittenType, java_shaped};
use crate::app::{App, Kind, Path, search};

impl App {
    pub(super) fn scala_tuple_returned(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        line: &str,
        name: &str,
    ) -> Option<WrittenType> {
        let call = search::scala_tuple_call(line, name)?;
        (call.receiver.is_some()
            || search::path_bindings(Kind::Jvm, file, text, line1, &call.method).is_empty())
        .then_some(())?;
        let decl = self.jvm_call_owner(file, text, line1, call.receiver.as_deref(), true)?;
        let found = self.jvm_hierarchy_member(&decl, &call.method, 0, &mut Vec::new());
        let [m] = java_shaped(found, |h| h, &call.method, true)
            .try_into()
            .ok()?;
        let written = search::scala_tuple_item(&m.text, &call.method, call.at, call.count)?;
        let t = self.text_of(&m.path)?;
        Some(WrittenType {
            resolved: self.jvm_type_at(&m.path, &t, &written),
            written,
        })
    }

    pub(super) fn scala_extracted(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        line: &str,
        name: &str,
    ) -> Option<WrittenType> {
        let (class, at, count) = search::scala_extracted(line, name)?;
        let JvmType::Project(decl) = self.jvm_type_on(file, text, &class, Some(line1)) else {
            return None;
        };
        let t = self.text_of(&decl.path)?;
        (!t.contains("def unapply")).then_some(())?;
        let written = search::scala_case_field(&decl.text, &class, at, count)?;
        Some(WrittenType {
            resolved: self.jvm_type_at(&decl.path, &t, &written),
            written,
        })
    }
}
