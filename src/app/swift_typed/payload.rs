use super::{App, Path, WrittenVia, search};

impl App {
    pub(super) fn swift_payload_value(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        payload: &search::SwiftPayload,
        hops: usize,
    ) -> Option<WrittenVia> {
        let names: Vec<String> = payload.subject.split('.').map(str::to_owned).collect();
        let held = self
            .swift_chain(here, file, text, line1, &names, hops.checked_sub(1)?)?
            .written;
        let ty = self
            .swift_type(here, &held)
            .filter(|t| t.keyword.as_deref() == Some("enum"))?;
        let generics = self.swift_decl_generics(here, &ty.name);
        if generics.is_empty() || generics.iter().any(|g| !g.is_empty()) {
            return None;
        }
        let rows = self.swift_member_rows(here, &ty, &payload.case)?;
        let [row] = rows.as_slice() else {
            return None;
        };
        if !search::swift_case(&row.text) {
            return None;
        }
        let written =
            search::swift_case_payload_type(&row.text, &payload.case, payload.at, payload.arity)?;
        let nested = format!("{}.{written}", ty.name);
        let written = if self.swift_type(here, &nested).is_some() {
            nested
        } else {
            written
        };
        Some(WrittenVia::plain(written))
    }
}
