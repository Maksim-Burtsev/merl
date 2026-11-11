use super::{App, Hit, Kind, Path, ProvenType, TypeVia, Typed, search};

impl App {
    pub(super) fn generic_base_field(
        &self,
        kind: Kind,
        ty: &Typed,
        field: &str,
    ) -> Option<TypeVia> {
        if !matches!(kind, Kind::TsJs | Kind::Go) {
            return None;
        }
        let (holder, text, binding) = self.hierarchy(kind, ty, 0, &mut |t| {
            let text = self.text_of(&t.path)?;
            let found = search::field_bindings(kind, &text, t.line, field);
            (!found.is_empty()).then(|| (t.clone(), text, found))
        })?;
        let [
            search::Binding {
                line1,
                value: search::Value::Type(written),
            },
        ] = binding.as_slice()
        else {
            return None;
        };
        let member = text.lines().nth(line1.checked_sub(1)?)?;
        let ty = self.generic_base_type(kind, ty, &holder, (member, field), written)?;
        Some(TypeVia {
            ty,
            call_signature: None,
        })
    }

    pub(super) fn generic_call_type(
        &self,
        kind: Kind,
        (file, text, line1): (&Path, &str, usize),
        receiver: &[String],
        method: &str,
        written: &str,
        hops: usize,
    ) -> Option<Typed> {
        let plain = written.chars().all(|c| c.is_alphanumeric() || c == '_');
        if !matches!(kind, Kind::TsJs | Kind::Go) || !plain || hops == 0 || receiver.is_empty() {
            return None;
        }
        let ProvenType { ty, .. } = self
            .chain_type(kind, file, text, line1, receiver, hops - 1)
            .ok()?;
        let (holder, found) = self.hierarchy(kind, &ty, 0, &mut |t| {
            let members = self.members_of(kind, t, method);
            (!members.is_empty()).then(|| (t.clone(), members))
        })?;
        let [decl] = <[Hit; 1]>::try_from(found).ok()?;
        let at = self.text_of(&decl.path)?;
        let member = at.lines().nth(decl.line1.checked_sub(1)?)?;
        self.generic_base_type(kind, &ty, &holder, (member, method), written)
    }

    pub(super) fn generic_base_type(
        &self,
        kind: Kind,
        ty: &Typed,
        holder: &Typed,
        (member, name): (&str, &str),
        written: &str,
    ) -> Option<Typed> {
        if !matches!(kind, Kind::TsJs | Kind::Go) || member.contains(&format!("{name}<")) {
            return None;
        }
        let text = self.text_of(&holder.path)?;
        let params = match search::go_receiver_parameters(member).filter(|_| kind == Kind::Go) {
            Some(params) => params,
            None => search::own_type_parameters(kind, &text, holder.line)?,
        };
        let index = params.iter().position(|p| p == written.trim())?;
        self.base_argument(kind, ty, holder, (index, params.len()), 0)
    }

    fn base_argument(
        &self,
        kind: Kind,
        ty: &Typed,
        holder: &Typed,
        at: (usize, usize),
        depth: usize,
    ) -> Option<Typed> {
        let text = self.text_of(&ty.path).filter(|_| depth < 8)?;
        let own = search::own_type_parameters(kind, &text, ty.line).unwrap_or_default();
        let mut found = None;
        for base in search::bases(kind, &text, ty.line) {
            let Some(b) = self.type_decl(kind, &ty.path, &base) else {
                continue;
            };
            let got = if (&b.path, b.line) == (&holder.path, holder.line) {
                let args = search::written_arguments(kind, &text, ty.line, &base)?;
                let arg = args.get(at.0).filter(|_| args.len() == at.1)?;
                let arg = arg
                    .strip_prefix('*')
                    .filter(|_| kind == Kind::Go)
                    .unwrap_or(arg);
                let plain = !arg.is_empty()
                    && arg
                        .chars()
                        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '.'));
                if !plain || own.iter().any(|p| p == arg) {
                    return None;
                }
                Some(self.type_decl(kind, &ty.path, arg)?)
            } else {
                self.base_argument(kind, &b, holder, at, depth + 1)
            };
            if got.is_some() {
                if found.is_some() {
                    return None;
                }
                found = got;
            }
        }
        found
    }
}
