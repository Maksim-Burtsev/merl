use super::{JvmType, TypedField};
use crate::app::{App, Kind, Path, Regex, search};

impl App {
    pub(super) fn kotlin_receiver_property(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        name: &str,
    ) -> Option<(JvmType, Vec<String>)> {
        let line = self.line + 1;
        let unbound = here.extension().is_some_and(|e| e == "kt" || e == "kts")
            && name.starts_with(|c: char| c.is_ascii_uppercase())
            && search::jvm_bindings_at(here, text, line, self.col, name).is_empty()
            && search::path_bindings(Kind::Jvm, here, text, line, name).is_empty()
            && search::jvm_enclosing_members(here, text, line, name).is_none()
            && search::kotlin_constructor_parameter(text, line, name).is_none();
        unbound.then_some(())?;
        let (owner, written) = match search::kotlin_receiver_scope(text, line, before) {
            Some(value) => {
                let w = self.jvm_value(here, text, line, &value, true, 1)?;
                (w.resolved, w.written)
            }
            None => {
                let ext = search::kotlin_extension_at(text, line, before)?;
                let written = bound_of(text, &ext.receiver, &ext.name).unwrap_or(ext.receiver);
                (self.jvm_type_at(here, text, &written), written)
            }
        };
        let JvmType::Project(decl) = owner else {
            return None;
        };
        let TypedField { hit, written: ty } = self.jvm_field_type(&decl, name, true)?;
        let hidden = ["private", "protected"];
        (!hit.text.split_whitespace().any(|w| hidden.contains(&w))).then_some(())?;
        let t = self.text_of(&hit.path)?;
        let resolved = self.jvm_type_on(&hit.path, &t, &ty, Some(hit.line1));
        Some((
            resolved,
            vec![format!("this: {written}"), format!("{name}: {ty}")],
        ))
    }
}

fn bound_of(text: &str, receiver: &str, name: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r"\bfun\s*<\s*{t}\s*:\s*([A-Z][\w.]*)\s*>\s*{t}\??\s*\.\s*{n}\b",
        t = regex::escape(receiver),
        n = regex::escape(name)
    ))
    .ok()?;
    let mut found = re.captures_iter(text).map(|c| c[1].to_owned());
    let one = found.next()?;
    found.next().is_none().then_some(one)
}
