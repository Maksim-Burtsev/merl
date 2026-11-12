use super::{App, Hit, Kind, Path, WrittenVia, search};

impl App {
    pub(super) fn swift_bare_call(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        method: &str,
        callee: &str,
    ) -> Option<WrittenVia> {
        if !search::bindings(Kind::Swift, text, line1, method).is_empty() {
            return None;
        }
        let pattern = search::def_patterns(Kind::Swift, method).join("|");
        let funcs: Vec<Hit> = self
            .project_definitions(Kind::Swift, here, method, &pattern)
            .into_iter()
            .filter(|h| {
                h.text.contains("func ")
                    && self.text_of(&h.path).is_some_and(|t| {
                        search::qualified(Kind::Swift, &t, h.line1, method)
                            .is_none_or(|q| q == *method)
                    })
            })
            .collect();
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let mut at = line1;
        let mut own = None;
        while let Some(up) = search::swift_enclosing_type(&lines, &literal, at) {
            let ty = self.swift_type(here, &search::swift_type_header(lines[up - 1])?.name)?;
            let rows = self.swift_member_rows(here, &ty, method)?;
            if !rows.is_empty() {
                own = Some((<[Hit; 1]>::try_from(rows).ok()?, ty.name));
                break;
            }
            at = up;
        }
        let (decl, owner) = match (own, <[Hit; 1]>::try_from(funcs)) {
            (Some(([decl], owner)), _) => (decl, Some(owner)),
            (None, Ok([decl])) => (decl, None),
            (None, Err(_)) => return None,
        };
        self.swift_returning(&decl, owner.clone(), callee)
            .or_else(|| {
                self.swift_base_returning(
                    here,
                    owner.as_deref()?,
                    std::slice::from_ref(&decl),
                    callee,
                )
            })
    }
}
