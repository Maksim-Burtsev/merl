use super::{JvmType, TypedField, WrittenType, java_shaped};
use crate::app::{App, Hit, Kind, Path, search};

impl App {
    pub(super) fn java_static_initialized(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
    ) -> Option<WrittenType> {
        let init = search::kotlin_initializer(text, line1, name)?;
        let init = init.trim().strip_suffix(';')?.trim_end();
        let (head, calls) = search::java_fluent_chain(&format!("{init}."))?;
        if !init.starts_with(&format!("{head}."))
            || !search::path_bindings(Kind::Jvm, file, text, line1, &head).is_empty()
            || search::jvm_enclosing_types(text, line1)
                .into_iter()
                .any(|d| !search::jvm_members_of(file, text, d, &head).is_empty())
        {
            return None;
        }
        let calls: Vec<String> = calls.into_iter().map(|c| format!("{c}()")).collect();
        let (ty, links, rest) = self.jvm_static_head(file, text, &head, &calls)?;
        let (resolved, links) = self.jvm_follow(ty, links, &head, rest, false)?;
        matches!(resolved, JvmType::Project(_)).then_some(())?;
        let written = links.last()?.rsplit(": ").next()?.to_owned();
        Some(WrittenType { resolved, written })
    }

    pub(super) fn java_lambda_element(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
    ) -> Option<WrittenType> {
        let Some(source) = search::java_lambda_source(text, line1, name) else {
            return self.java_lambda_parameter(file, text, line1, name);
        };
        if let Some((owner, method)) = source.reference {
            let JvmType::Project(decl) = self.jvm_type_at(file, text, &owner) else {
                return None;
            };
            let TypedField { hit, written } = self.jvm_call_type(&decl, &method, false)?;
            if !written.starts_with(|c: char| c.is_ascii_uppercase())
                || !written
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
            {
                return None;
            }
            let t = self.text_of(&hit.path)?;
            let resolved = self.jvm_type_at(&hit.path, &t, &written);
            return matches!(resolved, JvmType::Project(_))
                .then_some(WrittenType { resolved, written });
        }
        let (in_file, written) = match (source.receiver, source.call) {
            (Some(r), None) => {
                let (own, r) = match r.strip_prefix("this.") {
                    Some(field) => (true, field.to_owned()),
                    None => (false, r),
                };
                let mut bindings: Vec<usize> = if own {
                    Vec::new()
                } else {
                    search::path_bindings(Kind::Jvm, file, text, line1, &r)
                        .iter()
                        .map(|b| b.line1)
                        .collect()
                };
                if bindings.is_empty() {
                    let inner = *search::jvm_enclosing_types(text, line1).first()?;
                    search::jvm_type_name(text.lines().nth(inner.checked_sub(1)?)?)?;
                    let here = text.lines().nth(line1.checked_sub(1)?)?;
                    (!search::java_new_between(text, line1, here, inner)).then_some(())?;
                    (own || !search::java_local_above(text, line1, inner, &r)).then_some(())?;
                    bindings = search::jvm_members_of(file, text, inner, &r);
                }
                let [l] = bindings[..] else {
                    return None;
                };
                let at = text.lines().nth(l.checked_sub(1)?)?;
                (file.to_path_buf(), search::java_declared_element(at, &r)?)
            }
            (recv, Some((method, args))) => {
                let owner = match recv.as_deref() {
                    None | Some("this") => Self::jvm_own_type(file, text, line1, None)?,
                    Some(r) => self.jvm_value(file, text, line1, r, false, 0)?.resolved,
                };
                let decl = match owner {
                    JvmType::Project(decl) => decl,
                    JvmType::Outside(t) => {
                        let class = search::java_spring_body(text, &t, &method, &args)?;
                        let resolved = self.jvm_type_at(file, text, &class);
                        return matches!(resolved, JvmType::Project(_)).then_some(WrittenType {
                            resolved,
                            written: class,
                        });
                    }
                    JvmType::Unknown => return None,
                };
                let found = self.jvm_hierarchy_member(&decl, &method, 0, &mut Vec::new(), false);
                let found = java_shaped(found, |h| h, &method, true);
                if let Some(class) = search::java_class_argument(&args) {
                    let taking: Vec<&Hit> = (found.iter())
                        .filter(|h| search::java_takes_class(&h.text, &method))
                        .collect();
                    let exact = !taking.is_empty()
                        && (taking.iter())
                            .all(|h| search::java_returns_class_element(&h.text, &method));
                    exact.then_some((file.to_path_buf(), class))?
                } else {
                    let [hit] = &found[..] else {
                        return None;
                    };
                    let element = search::java_returned_element(&hit.text, &method)?;
                    (hit.path.clone(), element)
                }
            }
            (None, None) => return None,
        };
        let t = self.text_of(&in_file)?;
        let resolved = self.jvm_type_at(&in_file, &t, &written);
        matches!(resolved, JvmType::Project(_)).then_some(WrittenType { resolved, written })
    }

    fn java_lambda_parameter(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
    ) -> Option<WrittenType> {
        let search::JavaLambdaArgument {
            receiver,
            method,
            index,
        } = search::java_lambda_argument(text, line1, name)?;
        let owner = match receiver.as_deref() {
            None | Some("this") => Self::jvm_own_type(file, text, line1, None)?,
            Some(r) => self.jvm_value(file, text, line1, r, false, 0)?.resolved,
        };
        let JvmType::Project(decl) = owner else {
            return None;
        };
        let found = self.jvm_hierarchy_member(&decl, &method, 0, &mut Vec::new(), false);
        let [hit] = &java_shaped(found, |h| h, &method, true)[..] else {
            return None;
        };
        let t = self.text_of(&hit.path)?;
        let parameters = search::jvm_parameter_parts(&t, hit.line1, &method)?;
        let written = search::java_functional_parameter(parameters.get(index)?)?;
        let resolved = self.jvm_type_at(&hit.path, &t, &written);
        matches!(resolved, JvmType::Project(_)).then_some(WrittenType { resolved, written })
    }
}
