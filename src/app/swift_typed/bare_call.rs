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

    pub(super) fn swift_bare_closure_value(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        hops: usize,
    ) -> Option<WrittenVia> {
        let lines: Vec<&str> = text.lines().collect();
        let (method, at, count) = search::swift_bare_closure(lines.get(line1 - 1)?, name)?;
        if !search::bindings(Kind::Swift, text, line1, &method).is_empty() {
            return None;
        }
        let literal = search::literal_lines(Kind::Swift, text);
        let around = search::swift_enclosing_type(&lines, &literal, line1)?;
        let ty = self.swift_type(here, &search::swift_type_header(lines[around - 1])?.name)?;
        ty.keyword.as_ref()?;
        let mut inputs = None;
        let mut generics = self.swift_scope_generics(here, text, line1);
        for row in self.swift_member_rows(here, &ty, &method)? {
            if !row.text.contains("func ") {
                return None;
            }
            let params = search::swift_params(&row.text)?;
            let (last, before) = params.split_last()?;
            if before.iter().any(|p| {
                p.contains("->")
                    || search::swift_type_name(p)
                        .is_some_and(|n| self.swift_project_alias(here, &n))
            }) {
                return None;
            }
            generics.extend(search::swift_generics(&row.text));
            if last.is_empty() || last.contains("@autoclosure") {
                continue;
            }
            let bare = last
                .split_whitespace()
                .filter(|w| !w.starts_with('@'))
                .collect::<Vec<_>>()
                .join(" ");
            if search::swift_constrained_generic(&row.text, &bare) {
                continue;
            }
            let takes = if let Some(takes) = search::swift_function_inputs(last) {
                takes
            } else {
                let [alias] =
                    <[Hit; 1]>::try_from(self.swift_member_rows(here, &ty, &bare)?).ok()?;
                search::swift_function_inputs(&search::swift_alias(&alias.text, &bare)?)?
            };
            if inputs.as_ref().is_some_and(|i| *i != takes) {
                return None;
            }
            inputs = Some(takes);
        }
        let inputs = inputs.filter(|i| i.len() == count)?;
        let input = inputs.get(at)?;
        let named = search::swift_type_name(input).or_else(|| search::swift_element(input))?;
        if generics.contains(&named)
            || ["Element", "Index", "Indices", "Iterator", "SubSequence"].contains(&named.as_str())
        {
            return None;
        }
        let given = search::SwiftGiven::Type(input.clone());
        self.swift_given_type(here, file, text, line1, given, hops)
    }
}
