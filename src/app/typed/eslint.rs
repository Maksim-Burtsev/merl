use super::{App, Hit, Kind, Path, Typed, search};

impl App {
    pub(super) fn declares_interface(&self, kind: Kind, file: &Path, written: &str) -> bool {
        self.type_decl(kind, file, written).is_some_and(|ty| {
            self.is_interface(&ty)
                || self
                    .union_arms(kind, &ty)
                    .is_some_and(|arms| arms.iter().all(|a| self.is_interface(a)))
        })
    }

    pub(super) fn eslint_parent(
        &self,
        kind: Kind,
        here: &Path,
        text: &str,
        line1: usize,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Hit>> {
        let [name] = chain else { return None };
        if word != "parent" || !search::reads_jsdoc(kind, here) {
            return None;
        }
        let bindings = self.bindings_of(kind, here, text, line1, name);
        let visited = !bindings.is_empty()
            && bindings.iter().all(|b| {
                search::eslint_visited_node(text, b.line1, name).is_some_and(|n| n != "Program")
            });
        if !visited {
            return None;
        }
        let pattern = r"^\s*(?:export\s+)?(?:declare\s+)?interface\s+NodeParentExtension\b";
        let mut found = self.project_grep(kind, here, pattern);
        if let Some((_, files)) = self.external.get(&kind) {
            found.extend(self.external_grep(kind, files, pattern));
        }
        let [decl] = <[Hit; 1]>::try_from(found).ok()?;
        self.instance_member(
            kind,
            &Typed {
                name: "NodeParentExtension".to_owned(),
                path: decl.path,
                line: decl.line1,
            },
            word,
        )
    }
}
