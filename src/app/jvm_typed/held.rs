use std::ops::ControlFlow::{self, Break, Continue};

use super::{JvmType, java_shaped};
use crate::app::{App, Candidate, Hit, Kind, Path, search};

impl App {
    pub(super) fn jvm_held_shape(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
    ) -> ControlFlow<Option<Vec<Candidate>>> {
        let line1 = self.line + 1;
        let java = here.extension().is_some_and(|e| e == "java");
        if java && let Some(source) = search::java_element_access(before) {
            return Break(self.java_container_member(here, text, &source, word));
        }
        if java && let Some(source) = search::java_index_access(before) {
            let element = Self::jvm_declared_line(here, text, line1, &source)
                .and_then(|line| search::java_array_element(line, &source));
            return Break(element.and_then(|element| {
                let ty = self.jvm_type_at(here, text, &element);
                let label = format!("{source}: element {element}");
                self.jvm_typed_member(here, ty, &label, word, true)
            }));
        }
        if java && let Some((made, links)) = search::java_constructed_chain(before) {
            let ty = self.jvm_made(here, text, &made, false);
            let followed = self.jvm_follow(ty, vec![format!("new {made}()")], &made, &links, true);
            return Break(followed.and_then(|(ty, links)| {
                self.jvm_typed_member(here, ty, &links.join(" \u{2192} "), word, true)
            }));
        }
        let kt = here.extension().is_some_and(|e| e == "kt" || e == "kts");
        if kt
            && let Some(source) = search::kotlin_map_read(before)
            && let Some(line) = Self::jvm_declared_line(here, text, line1, &source)
            && let Some((container, held, label)) = search::kotlin_map_value(line, &source)
                .map(|(c, v)| (c, v, "value"))
                .or_else(|| {
                    search::kotlin_list_element(line, &source).map(|(c, e)| (c, e, "element"))
                })
        {
            let label = format!("{source}: {label} {held}");
            return Break(self.jvm_held_member(here, text, &container, &held, &label, word));
        }
        Continue(())
    }

    fn java_container_member(
        &self,
        here: &Path,
        text: &str,
        source: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let line = Self::jvm_declared_line(here, text, self.line + 1, source)?;
        let (container, element) = search::java_element_type(line, source)?;
        let label = format!("{source}: element {element}");
        self.jvm_held_member(here, text, &container, &element, &label, word)
    }

    fn jvm_held_member(
        &self,
        here: &Path,
        text: &str,
        container: &str,
        held: &str,
        label: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        if let JvmType::Project(hit) = self.jvm_type_at(here, text, container)
            && self.jvm_sees(here, text, container, &hit)
        {
            return None;
        }
        let ty = self.jvm_type_at(here, text, held);
        self.jvm_typed_member(here, ty, label, word, true)
    }

    pub(super) fn java_returned_element(
        &self,
        owner: &Hit,
        name: &str,
        instance: bool,
        read: &str,
    ) -> Option<(String, JvmType)> {
        search::java_element_read(read).then_some(())?;
        let found = self.jvm_hierarchy_member(owner, name, 0, &mut Vec::new(), instance);
        let found = java_shaped(found, |h| h, name, true);
        let (method, more) = found.split_first()?;
        method
            .path
            .extension()
            .is_some_and(|e| e == "java")
            .then_some(())?;
        let (_, element) = search::java_returned_held(&method.text, name)?;
        more.iter()
            .all(|h| {
                h.path == method.path
                    && search::java_returned_held(&h.text, name).is_some_and(|(_, e)| e == element)
            })
            .then_some(())?;
        let text = self.text_of(&method.path)?;
        let ty = self.jvm_type_on(&method.path, &text, &element, Some(method.line1));
        Some((element, ty))
    }

    pub(super) fn jvm_element(file: &Path, text: &str, line1: usize, name: &str) -> Option<String> {
        let line = Self::jvm_declared_line(file, text, line1, name)?;
        search::kotlin_element_type(line, name)
    }

    pub(super) fn jvm_declared_line<'a>(
        file: &Path,
        text: &'a str,
        line1: usize,
        name: &str,
    ) -> Option<&'a str> {
        let mut declared: Vec<usize> = search::path_bindings(Kind::Jvm, file, text, line1, name)
            .iter()
            .map(|b| b.line1)
            .collect();
        if declared.is_empty() {
            declared = search::jvm_enclosing_types(text, line1)
                .into_iter()
                .map(|d| search::jvm_members_of(file, text, d, name))
                .find(|m| !m.is_empty())?;
        }
        let [l] = declared[..] else {
            return None;
        };
        text.lines().nth(l.checked_sub(1)?)
    }
}
