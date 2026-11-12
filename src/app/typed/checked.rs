use super::{App, Kind, Path, TypeVia, search};

impl App {
    pub(super) fn checked_or(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        declared: &dyn Fn() -> Option<TypeVia>,
    ) -> Option<TypeVia> {
        if !matches!(kind, Kind::TsJs | Kind::Python) {
            return declared();
        }
        let (written, guard) = match search::checked_type(kind, text, line1, name) {
            None => return declared(),
            Some(search::Checked::Unreadable) => return None,
            Some(search::Checked::To(written, guard)) => (written, guard),
        };
        if !name.contains('.')
            && self
                .bindings_of(kind, file, text, line1, name)
                .iter()
                .any(|b| b.line1 >= guard)
        {
            return declared();
        }
        let checked = self.type_decl(kind, file, &written)?;
        let at = (checked.path.clone(), checked.line);
        match declared() {
            Some(d)
                if self
                    .hierarchy(kind, &d.ty, 0, &mut |t| {
                        ((t.path.clone(), t.line) == at).then_some(())
                    })
                    .is_some() =>
            {
                Some(d)
            }
            _ => Some(TypeVia {
                ty: checked,
                call_signature: None,
            }),
        }
    }
}
