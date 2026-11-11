use super::super::typed::{Base, base_argument};
use super::{App, Hit, Kind, Path, WrittenVia, search};

impl App {
    pub(super) fn swift_base_returning(
        &self,
        here: &Path,
        owner: &str,
        rows: &[Hit],
        callee: &str,
    ) -> Option<WrittenVia> {
        let [decl] = rows else {
            return None;
        };
        if !decl.text.contains("func ") {
            return None;
        }
        let text = self.text_of(&decl.path)?;
        let returns = search::swift_returns(&text, decl.line1)?;
        let returns = returns.trim();
        let lines: Vec<&str> = text.lines().collect();
        if search::swift_generic_params(&lines, decl.line1)?
            .iter()
            .any(|p| p == returns)
        {
            return None;
        }
        let literal = search::literal_lines(Kind::Swift, &text);
        let holder = search::swift_scope(&lines, &literal, decl.line1).header_line1;
        let header = search::swift_type_header(lines.get(holder.checked_sub(1)?)?)?;
        if header.keyword != "class" {
            return None;
        }
        let params = search::generic_header(Kind::Swift, &text, holder, &header.name)?.params;
        let index = params.iter().position(|p| p == returns)?;
        let start = self.swift_class_decl(here, owner)?;
        let (sub, arg) = base_argument(&start, (index, params.len()), 0, &|t: &Hit| {
            let text = self.text_of(&t.path)?;
            let name = search::swift_type_header(&t.text)?.name;
            let own = search::generic_header(Kind::Swift, &text, t.line1, &name)?;
            let bases = search::swift_inherited(&t.text)
                .into_iter()
                .filter_map(|b| {
                    let d = self.swift_class_decl(&t.path, &b)?;
                    Some(if (&d.path, d.line1) == (&decl.path, holder) {
                        Base::Holder(search::base_arguments(&own.rest, &b))
                    } else {
                        Base::Other(d)
                    })
                })
                .collect();
            Some((own.params, bases))
        })?;
        self.swift_type(&sub.path, &arg)?;
        Some(WrittenVia {
            call_signature: Some(format!("{callee}() -> {arg}")),
            written: arg,
        })
    }

    fn swift_class_decl(&self, here: &Path, name: &str) -> Option<Hit> {
        if name.contains('.') || self.swift_type(here, name)?.keyword.as_deref() != Some("class") {
            return None;
        }
        let pattern = search::def_patterns(Kind::Swift, name).join("|");
        let decls: Vec<Hit> = self
            .project_definitions(Kind::Swift, here, name, &pattern)
            .into_iter()
            .filter(|h| {
                self.in_code(Kind::Swift, h)
                    && search::swift_type_header(&h.text)
                        .is_some_and(|t| t.keyword == "class" && t.name == name)
            })
            .collect();
        <[Hit; 1]>::try_from(decls).ok().map(|[d]| d)
    }
}
