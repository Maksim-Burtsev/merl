use super::JvmType;
use crate::app::{App, Hit, Path, Regex, search};

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

    pub(super) fn kotlin_unseen_here(&self, file: &Path, text: &str, hit: &Hit) -> bool {
        static CLASS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^\s*(?:(?:private|internal|public|protected|data|sealed|abstract|open|final)\s+)*class\s+\w")
                .expect("the regex is a literal")
        });
        static NESTED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^\s*(?:(?:private|internal|public|protected|data|sealed|abstract|open|final)\s+)*(?:companion\s+object|class|object)\b[^:(]*\{\s*$")
                .expect("the regex is a literal")
        });
        let kotlin = file.extension().is_some_and(|e| e == "kt" || e == "kts");
        if !kotlin || hit.path != file || self.rel_current().as_deref() != Some(file) {
            return false;
        }
        let line1 = self.line + 1;
        let lines: Vec<&str> = text.lines().collect();
        let header = |t: usize| lines.get(t - 1).copied().unwrap_or_default();
        let around = search::jvm_enclosing_types(text, line1);
        let owner = search::jvm_enclosing_types(text, hit.line1)
            .first()
            .copied();
        let Some(k) = around.iter().position(|&t| Some(t) == owner) else {
            return false;
        };
        k > 0
            && search::kotlin_member_scope(text, line1, self.line_str()) == around.first().copied()
            && CLASS.is_match(header(around[k]))
            && around[..k]
                .iter()
                .all(|&t| NESTED.is_match(header(t)) && search::jvm_local_block(text, t).is_none())
    }
}

pub(super) fn field_elsewhere(file: &Path, hit: &Hit, name: &str) -> bool {
    hit.path != file
        && hit.path.extension().is_some_and(|e| e == "java")
        && search::jvm_declared_type(&hit.text, name, false).is_some()
}
