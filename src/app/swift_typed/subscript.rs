use super::{App, Candidate, Path, Reason, search};

impl App {
    pub(in crate::app) fn swift_subscript_rows(
        &self,
        here: &Path,
        text: &str,
        lead: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let indexed = lead.trim_end_matches(['?', '!']);
        let receiver = search::swift_subscript_receiver(&format!("{indexed}."))?;
        let open = indexed.rfind('[')?;
        let line = text.lines().nth(self.line)?;
        let root = receiver.split('.').next()?;
        if indexed.get(open..)?.contains("..") || search::swift_closure_param(line, root, self.col)
        {
            return None;
        }
        let names: Vec<String> = receiver.split('.').map(str::to_owned).collect();
        let held = self
            .swift_chain(here, here, text, self.line + 1, &names, 3)?
            .written;
        let element = search::swift_element(&held)
            .or_else(|| search::swift_dictionary_part(&held, "values"))?;
        let ty = self
            .swift_type(here, &element)
            .filter(|t| t.keyword.is_some())?;
        let mut rows = self.swift_member_rows(here, &ty, word)?;
        rows.retain(|h| {
            search::swift_member_kind(&h.text) != Some(search::SwiftMember::StaticOrClass)
        });
        let label = format!("{receiver}[…].{word}: {}", ty.name);
        Some(
            rows.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect(),
        )
    }
}
