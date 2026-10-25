use super::{App, Candidate, Hit, Kind, Path, Reason, search};

impl App {
    pub(super) fn returned_key(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let [name] = chain else {
            return None;
        };
        if !search::reads_jsdoc(kind, here) {
            return None;
        }
        let (file, text, bindings, _) = self.last_bindings(kind, here, chain)?;
        let mut calls = Vec::new();
        for b in &bindings {
            match &b.value {
                search::Value::Call(callee) => calls.push((b.line1, callee.clone())),
                search::Value::Unknown => calls.extend(
                    search::js_assigned_calls(&text, b.line1, name)?
                        .into_iter()
                        .map(|c| (b.line1, c)),
                ),
                _ => return None,
            }
        }
        let mut found: Option<(Hit, String)> = None;
        for (at_line, callee) in calls {
            let decl = self.callee_decl(kind, &file, &text, at_line, &callee, 1)?;
            let at = self.text_of(&decl.path)?;
            let line1 = search::reads_jsdoc(kind, &decl.path)
                .then(|| search::jsdoc_returned_key(&at, decl.line1, word))??;
            if found
                .as_ref()
                .is_some_and(|(h, _)| (&h.path, h.line1) != (&decl.path, line1))
            {
                return None;
            }
            let hit = Hit {
                text: at.lines().nth(line1 - 1)?.to_owned(),
                path: decl.path,
                line1,
                byte_col: None,
                deleted: None,
            };
            found = Some((hit, format!("{name} = {callee}()")));
        }
        let (hit, link) = found?;
        Some(vec![Candidate {
            hit,
            reason: Reason::Receiver(link),
        }])
    }
}
