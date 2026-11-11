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

pub(super) fn forget_swapped(
    text: &str,
    line1: usize,
    name: &str,
    bindings: &mut [search::Binding],
) {
    let lines: Vec<&str> = text.lines().collect();
    for b in bindings {
        let unpacked = lines
            .get(b.line1.wrapping_sub(1))
            .and_then(|l| assigned_names(l))
            .is_some_and(|lhs| {
                let lhs = lhs.trim_start().split(':').next().unwrap_or_default();
                lhs.contains([',', '[']) && !lhs.get(1..).unwrap_or_default().contains('(')
            });
        let again = || {
            lines
                .get(b.line1..line1.saturating_sub(1))
                .unwrap_or_default()
                .iter()
                .filter_map(|l| assigned_names(l))
                .any(|lhs| {
                    lhs.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                        .any(|w| w == name)
                })
        };
        if unpacked && again() {
            b.value = search::Value::Unknown;
        }
    }
}

fn assigned_names(line: &str) -> Option<&str> {
    let bytes = line.as_bytes();
    let at = (0..bytes.len()).find(|&i| {
        bytes[i] == b'='
            && !matches!(bytes.get(i + 1), Some(b'=' | b'>'))
            && !matches!(
                i.checked_sub(1).map(|p| bytes[p]),
                Some(b'=' | b'!' | b'<' | b'>')
            )
    })?;
    line.get(..at)
}
