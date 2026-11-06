use super::{App, Candidate, Path, Reason, search};

impl App {
    pub(in crate::app) fn swift_key_path_rows(
        &self,
        here: &Path,
        text: &str,
        lead: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        static CALL: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(r"(?:^|[^\w.)\]?!])([A-Za-z_][\w.]*)\.([A-Za-z_]\w*)\(\s*\\$")
                .expect("the regex is a literal")
        });
        let c = CALL.captures(lead)?;
        let call = (c[1].to_owned(), c[2].to_owned());
        let root = self.swift_closure_value(here, here, text, self.line + 1, call, 3)?;
        let ty = self.swift_type(here, &root.written)?;
        let mut rows = self.swift_member_rows(here, &ty, word)?;
        rows.retain(|h| {
            search::swift_member_kind(&h.text) != Some(search::SwiftMember::StaticOrClass)
        });
        let label = format!("\\.{word}: {}", ty.name);
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
