use super::{App, Hit, Kind, Path, ProvenType, TypeVia, search, signature, ts_declared_param};

impl App {
    pub(super) fn calls_type(
        &self,
        kind: Kind,
        here: &Path,
        line: &str,
        word_start: usize,
    ) -> Option<Result<ProvenType, String>> {
        let (end, methods) = search::go_method_calls(line, word_start)?;
        let head = search::call_head(kind, line, end)?;
        let ProvenType { mut ty, mut links } = self.receiver(kind, here, &[], Some(&head)).ok()?;
        for method in &methods {
            let (holder, found) = self.hierarchy(kind, &ty, 0, &mut |t| {
                let members = self.members_of(kind, t, method);
                (!members.is_empty()).then(|| (t.clone(), members))
            })?;
            let [decl] = <[Hit; 1]>::try_from(found).ok()?;
            let text = self.text_of(&decl.path)?;
            let search::Value::Type(written) =
                self.returns_of(kind, &decl.path, &text, decl.line1)?
            else {
                return None;
            };
            links.push(signature(kind, method, &written));
            if kind == Kind::TsJs && written.trim() == "this" {
                continue;
            }
            if kind == Kind::Python
                && written.trim() == "Self"
                && super::super::python::imports_typing_self(&text)
            {
                continue;
            }
            let member = (
                text.lines().nth(decl.line1 - 1).unwrap_or(""),
                method.as_str(),
            );
            match self
                .generic_base_type(kind, &ty, &holder, member, &written)
                .or_else(|| {
                    if ts_declared_param(kind, &text, decl.line1, &written) {
                        return None;
                    }
                    self.annotated_type(kind, &decl.path, &text, 0, &written)
                }) {
                Some(next) => ty = next,
                None if kind == Kind::Go && self.go_outside(&decl.path, &text, &written) => {
                    return Some(Err(links.join(" \u{2192} ")));
                }
                None => return None,
            }
        }
        Some(Ok(ProvenType { ty, links }))
    }

    pub(super) fn assigned_calls_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
    ) -> Option<TypeVia> {
        if self.rel_current().as_deref() != Some(file) {
            return None;
        }
        let name = text.lines().nth(line1.checked_sub(1)?)?.trim_start();
        let name = name.get(..name.find(|c: char| !(c.is_alphanumeric() || c == '_'))?)?;
        let expr = match kind {
            Kind::Go => search::go_assigned_chain(text, line1, name)?,
            _ => search::python_assigned_call(text, line1, name)?,
        };
        let probe = format!("{expr}.x");
        let ProvenType { ty, links } = self.calls_type(kind, file, &probe, expr.len() + 1)?.ok()?;
        Some(TypeVia {
            ty,
            call_signature: Some(links.join(" \u{2192} ")),
        })
    }
}
