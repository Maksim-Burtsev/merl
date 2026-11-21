use super::{JvmType, TypedField};
use crate::app::{App, Hit, Kind, Path, Regex, search};

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
        let block = || {
            let (recv, callee) = search::kotlin_block_call(text, line, before)?;
            self.kotlin_block_receiver(here, text, recv.as_deref(), &callee)
        };
        let (owner, written) =
            if let Some(value) = search::kotlin_receiver_scope(text, line, before) {
                let w = self.jvm_value(here, text, line, &value, true, 1)?;
                (w.resolved, w.written)
            } else if let Some(found) = block() {
                found
            } else if let Some(ext) = search::kotlin_extension_at(text, line, before) {
                let written = bound_of(text, &ext.receiver, &ext.name).unwrap_or(ext.receiver);
                (self.jvm_type_at(here, text, &written), written)
            } else {
                let ty = search::kotlin_member_scope(text, line, before)?;
                let at = text.lines().nth(ty - 1)?;
                let written = search::jvm_type_name(at)?;
                let decl = Hit {
                    deleted: None,
                    path: here.to_path_buf(),
                    line1: ty,
                    byte_col: None,
                    text: at.to_owned(),
                };
                (JvmType::Project(decl), written)
            };
        let JvmType::Project(decl) = owner else {
            return None;
        };
        let TypedField { hit, written: ty } = self.jvm_field_type(&decl, name, true)?;
        let hidden = ["private", "protected"];
        let value = search::jvm_type_name(&hit.text).as_deref() != Some(name);
        (value && !hit.text.split_whitespace().any(|w| hidden.contains(&w))).then_some(())?;
        let t = self.text_of(&hit.path)?;
        let resolved = self.jvm_type_on(&hit.path, &t, &ty, Some(hit.line1));
        Some((
            resolved,
            vec![format!("this: {written}"), format!("{name}: {ty}")],
        ))
    }

    fn kotlin_block_receiver(
        &self,
        here: &Path,
        text: &str,
        recv: Option<&str>,
        callee: &str,
    ) -> Option<(JvmType, String)> {
        let line = self.line + 1;
        (search::path_bindings(Kind::Jvm, here, text, line, callee).is_empty()
            && search::jvm_enclosing_members(here, text, line, callee).is_none())
        .then_some(())?;
        let hit = self.kotlin_seen_fun(here, text, line, callee, true)?;
        let f = search::kotlin_block_fun(&hit.text, callee)?;
        let generic = f.type_parameter.as_deref() == Some(f.block.as_str());
        match (recv, f.extension) {
            (Some(r), Some(ext)) if generic && ext == f.block => {
                let w = self.jvm_value(here, text, line, r, true, 1)?;
                if let JvmType::Project(d) = &w.resolved {
                    let member = self.jvm_hierarchy_member(d, callee, 0, &mut Vec::new(), true);
                    member.is_empty().then_some(())?;
                }
                Some((w.resolved, w.written))
            }
            (None, None) if !generic => {
                let t = self.text_of(&hit.path)?;
                let resolved = self.jvm_type_on(&hit.path, &t, &f.block, Some(hit.line1));
                Some((resolved, f.block))
            }
            _ => None,
        }
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
