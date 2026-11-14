use super::{App, Hit, Kind, Path, search};

impl App {
    pub(super) fn jsdoc_intersection_member(
        &self,
        kind: Kind,
        here: &Path,
        text: &str,
        line1: usize,
        chain: &[String],
        word: &str,
    ) -> Option<(Vec<Hit>, String)> {
        let [name] = chain else { return None };

        if !search::reads_jsdoc(kind, here) {
            return None;
        }
        let [
            search::Binding {
                value: search::Value::Call(callee),
                ..
            },
        ] = &self.bindings_of(kind, here, text, line1, name)[..]
        else {
            return None;
        };
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        let decl = self.declaration(kind, here, &parts)?;
        if !search::reads_jsdoc(kind, &decl.path) {
            return None;
        }
        let written = search::jsdoc_returns_written(&self.text_of(&decl.path)?, decl.line1)?;
        let mut found = None;
        for arm in search::ts_intersection_names(&written)? {
            let ty = self.type_decl(kind, &decl.path, &arm)?;
            if let Some(hits) =
                self.hierarchy(kind, &ty, 0, &mut |t| self.instance_member(kind, t, word))
                && found.replace(hits).is_some()
            {
                return None;
            }
        }
        Some((
            found?,
            format!("{name} \u{2192} {callee}() returns {written}"),
        ))
    }
}
