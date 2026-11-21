use super::{JvmType, WrittenType, java_shaped};
use crate::app::{App, Hit, Kind, Path, search};

impl App {
    pub(super) fn scala_tuple_returned(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        line: &str,
        name: &str,
    ) -> Option<WrittenType> {
        if let Some(t) = search::scala_tuple_literal(line, name) {
            return self.scala_project_type(file, text, line1, t);
        }
        let call = match search::scala_tuple_arm(line, name) {
            Some((at, count)) => {
                let subject = search::scala_arm_subject(text, line1)?;
                search::ScalaTupleCall {
                    receiver: None,
                    method: subject
                        .strip_suffix(')')?
                        .trim_end()
                        .strip_suffix('(')?
                        .trim_end()
                        .to_owned(),
                    at,
                    count,
                }
            }
            None => search::scala_tuple_call(line, name)?,
        };
        if call.receiver.is_none() {
            match search::path_bindings(Kind::Jvm, file, text, line1, &call.method)[..] {
                [] => {}
                [ref b] => {
                    let def = text.lines().nth(b.line1.checked_sub(1)?)?;
                    let t = search::scala_tuple_item(def, &call.method, call.at, call.count)
                        .or_else(|| {
                            search::scala_tuple_body(def, &call.method, call.at, call.count)
                        })?;
                    return self.scala_project_type(file, text, b.line1, t);
                }
                _ => return None,
            }
        }
        let decl = match call.receiver.as_deref() {
            Some(r) if r.starts_with(|c: char| c.is_ascii_uppercase()) => {
                match self.jvm_type_on(file, text, r, Some(line1)) {
                    JvmType::Project(decl) => decl,
                    _ => return None,
                }
            }
            r => self.jvm_call_owner(file, text, line1, r, true)?,
        };
        let found = self.jvm_hierarchy_member(&decl, &call.method, 0, &mut Vec::new(), false);
        let [m] = java_shaped(found, |h| h, &call.method, true)
            .try_into()
            .ok()?;
        let t = self.text_of(&m.path)?;
        let next = t.lines().nth(m.line1).unwrap_or_default();
        let written = search::scala_tuple_item(&m.text, &call.method, call.at, call.count)
            .or_else(|| {
                let joined = format!("{} {}", m.text.trim_end(), next.trim());
                search::scala_tuple_item(&joined, &call.method, call.at, call.count)
            })?;
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
        if class == "Some" {
            return (count == 1)
                .then(|| self.scala_some_arm(file, text, line1))
                .flatten();
        }
        let decl = match self.jvm_type_on(file, text, &class, Some(line1)) {
            JvmType::Project(decl) => decl,
            JvmType::Unknown => self.scala_case_class(file, text, &class, at, count)?,
            JvmType::Outside(_) => return None,
        };
        let t = self.text_of(&decl.path)?;
        (!t.contains("def unapply")).then_some(())?;
        let written = search::scala_case_field(&decl.text, &class, at, count)?;
        Some(WrittenType {
            resolved: self.jvm_type_at(&decl.path, &t, &written),
            written,
        })
    }

    fn scala_case_class(
        &self,
        file: &Path,
        text: &str,
        class: &str,
        at: usize,
        count: usize,
    ) -> Option<Hit> {
        let pattern = search::def_patterns(Kind::Jvm, class).join("|");
        let cut = self.truncated.get();
        let hits = self.project_definitions(Kind::Jvm, file, class, &pattern);
        let full = self.truncated.get() && !cut;
        self.truncated.set(cut);
        (!full).then_some(())?;
        let [decl]: [Hit; 1] = hits
            .into_iter()
            .filter(|h| {
                search::scala(&h.path)
                    && search::scala_case_field(&h.text, class, at, count).is_some()
                    && self.jvm_code(h)
                    && self.jvm_sees(file, text, class, h)
            })
            .collect::<Vec<_>>()
            .try_into()
            .ok()?;
        Some(decl)
    }

    fn scala_some_arm(&self, file: &Path, text: &str, line1: usize) -> Option<WrittenType> {
        let subject = search::scala_arm_subject(text, line1)?;
        let [ref b] = search::path_bindings(Kind::Jvm, file, text, line1, &subject)[..] else {
            return None;
        };
        let t = search::scala_option_of(text.lines().nth(b.line1.checked_sub(1)?)?, &subject)?;
        self.scala_project_type(file, text, b.line1, t)
    }

    fn scala_project_type(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        written: String,
    ) -> Option<WrittenType> {
        match self.jvm_type_on(file, text, &written, Some(line1)) {
            resolved @ JvmType::Project(_) => Some(WrittenType { resolved, written }),
            _ => None,
        }
    }
}
