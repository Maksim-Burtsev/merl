use super::{App, Hit, WrittenVia, search};

impl App {
    pub(super) fn swift_labelled_returning(
        &self,
        e: &str,
        rows: &[Hit],
        owner: &str,
        callee: &str,
    ) -> Option<WrittenVia> {
        let call = search::swift_written_labels(e)?;
        let method = callee.rsplit('.').next()?;
        let fits: Vec<bool> = rows
            .iter()
            .map(|h| {
                let text = self.text_of(&h.path)?;
                let lines: Vec<&str> = text.lines().collect();
                let params = search::swift_param_labels(&lines, h.line1.checked_sub(1)?, method)?;
                Some(search::swift_call_fits(&call, &params))
            })
            .collect::<Option<_>>()?;
        let fitting: Vec<Hit> = rows
            .iter()
            .zip(&fits)
            .filter(|(_, fit)| **fit)
            .map(|(h, _)| h.clone())
            .collect();
        if fitting.is_empty() || fitting.len() == rows.len() {
            return None;
        }
        self.swift_overloads_returning(&fitting, owner, callee)
    }
}
