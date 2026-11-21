use super::JvmType;
use crate::app::{App, Hit, Path, search};

impl App {
    pub(super) fn jvm_type_written(
        &self,
        file: &Path,
        text: &str,
        written: &str,
        on: Option<usize>,
    ) -> JvmType {
        let java = file.extension().is_some_and(|e| e == "java");
        self.jvm_type_hop(file, text, written, on, 8, java)
    }
}

pub(super) fn field_elsewhere(file: &Path, hit: &Hit, name: &str) -> bool {
    hit.path != file
        && hit.path.extension().is_some_and(|e| e == "java")
        && search::jvm_declared_type(&hit.text, name, false).is_some()
}
