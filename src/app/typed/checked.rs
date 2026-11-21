use super::{App, Kind, Path, TypeVia, Typed, search};

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
        if kind == Kind::TsJs
            && let Some(via) = self.else_of_check(kind, file, text, line1, name)
        {
            return Some(via);
        }
        let Some((written, guard)) = search::checked_type(kind, text, line1, name) else {
            return declared();
        };
        if !name.contains('.')
            && self
                .bindings_of(kind, file, text, line1, name)
                .iter()
                .any(|b| b.line1 >= guard)
        {
            return declared();
        }
        let Some(checked) = self.type_decl(kind, file, &written) else {
            return declared();
        };
        match declared() {
            Some(d) if !self.derives(kind, &checked, &d.ty, 0) => Some(d),
            _ => Some(TypeVia {
                ty: checked,
                call_signature: None,
            }),
        }
    }

    fn else_of_check(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
    ) -> Option<TypeVia> {
        if name.contains('.') {
            return None;
        }
        let (checked, guard) = search::ts_else_of_check(text, line1, name)?;
        let [
            search::Binding {
                line1: bound,
                value: search::Value::Type(written),
            },
        ] = &self.bindings_of(kind, file, text, line1, name)[..]
        else {
            return None;
        };
        if *bound >= guard {
            return None;
        }
        let other = match &search::ts_union_names(written)?[..] {
            [a, b] if *a == checked => b.clone(),
            [a, b] if *b == checked => a.clone(),
            _ => return None,
        };
        self.type_decl(kind, file, &other).map(|ty| TypeVia {
            ty,
            call_signature: None,
        })
    }

    fn derives(&self, kind: Kind, ty: &Typed, from: &Typed, depth: usize) -> bool {
        if (&ty.path, ty.line) == (&from.path, from.line) {
            return true;
        }
        let Some(text) = self.text_of(&ty.path).filter(|_| depth < 8) else {
            return false;
        };
        search::bases(kind, &text, ty.line)
            .into_iter()
            .chain(search::interfaces(kind, &text, ty.line))
            .filter_map(|b| self.type_decl(kind, &ty.path, &b))
            .any(|b| self.derives(kind, &b, from, depth + 1))
    }
}
