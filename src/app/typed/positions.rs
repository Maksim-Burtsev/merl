use super::{App, Kind, Path, TypeVia, search, signature};

impl App {
    pub(super) fn call_position_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        b: &search::Binding,
        hops: usize,
    ) -> Option<TypeVia> {
        let search::Value::Position { of, at, count } = &b.value else {
            return None;
        };
        let search::Value::Call(callee) = &**of else {
            return None;
        };
        let (at, count) = (*at, *count);
        let decl = self.callee_decl(kind, file, text, b.line1, callee, hops)?;
        if search::declares_type(kind, &decl.text) {
            return None;
        }
        let src = self.text_of(&decl.path)?;
        let written = search::returned_element(kind, &src, decl.line1, at, count)?;
        if search::go_result_type_param(&decl.text, &written).is_some() {
            return None;
        }
        let ty = self.annotated_type(kind, &decl.path, &src, decl.line1, &written)?;
        Some(TypeVia {
            ty: self.runtime_class(ty),
            call_signature: Some(signature(kind, callee, &written)),
        })
    }

    pub(super) fn dict_value(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        receiver: &str,
        method: &str,
    ) -> Option<TypeVia> {
        let kind = Kind::Python;
        let mut found: Option<TypeVia> = None;
        for b in self.bindings_of(kind, file, text, line1, receiver) {
            let search::Value::Type(written) = &b.value else {
                return None;
            };
            let value = search::python_dict_value(written)?;
            let ty = self.annotated_type(kind, file, text, b.line1, &value)?;
            match &found {
                Some(one) if (&one.ty.path, one.ty.line) != (&ty.path, ty.line) => return None,
                Some(_) => {}
                None => {
                    found = Some(TypeVia {
                        ty,
                        call_signature: Some(format!("{receiver}.{method}() -> {value}")),
                    });
                }
            }
        }
        found
    }
}
