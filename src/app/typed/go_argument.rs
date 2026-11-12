use super::{App, Kind, Path, search};

impl App {
    pub(super) fn go_type_argument(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        callee: &str,
        params: &[String],
        type_param: &str,
    ) -> Option<String> {
        let line = text.lines().nth(line1.checked_sub(1)?)?;
        let args = search::go_call_arguments(line, callee)?;
        let variadic = params.last().is_some_and(|p| p.starts_with("..."));
        if args.len() != params.len() && !(variadic && args.len() + 1 >= params.len()) {
            return None;
        }
        let mut found: Option<String> = None;
        for (i, arg) in args.iter().enumerate() {
            let param = params
                .get(i)
                .or_else(|| params.last().filter(|_| variadic))?;
            if !arg.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let bindings = self.bindings_of(Kind::Go, file, text, line1, arg);
            let [
                search::Binding {
                    value: search::Value::Type(written),
                    ..
                },
            ] = bindings.as_slice()
            else {
                continue;
            };
            if let Some(t) = search::go_type_argument(param, written, type_param)
                && *found.get_or_insert_with(|| t.clone()) != t
            {
                return None;
            }
        }
        found
    }

    pub(in crate::app) fn go_field_argument(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        (var, field): (&str, &str),
    ) -> Option<String> {
        let bindings = self.bindings_of(Kind::Go, file, text, line1, var);
        let [
            search::Binding {
                value: search::Value::Type(written),
                ..
            },
        ] = bindings.as_slice()
        else {
            return None;
        };
        let (name, args) = search::go_type_with_arguments(written)?;
        let ty = self.type_decl(Kind::Go, file, name)?;
        let decl = self.text_of(&ty.path)?;
        let fields = search::field_bindings(Kind::Go, &decl, ty.line, field);
        let [
            search::Binding {
                value: search::Value::Type(held),
                ..
            },
        ] = fields.as_slice()
        else {
            return None;
        };
        let params = search::own_type_parameters(Kind::Go, &decl, ty.line)?;
        let i = params.iter().position(|p| p == held.trim())?;
        args.get(i).filter(|_| args.len() == params.len()).cloned()
    }
}
