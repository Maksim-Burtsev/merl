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
        let (sub, arg) = base_argument(ty, (index, params.len()), 0, &|t: &Typed| {
            let text = self.text_of(&t.path)?;
            let own = search::own_type_parameters(kind, &text, t.line).unwrap_or_default();
            let bases = search::bases(kind, &text, t.line)
                .into_iter()
                .filter_map(|base| {
                    let b = self.type_decl(kind, &t.path, &base)?;
                    if (&b.path, b.line) != (&holder.path, holder.line) {
                        return Some(Base::Other(b));
                    }
                    let args = search::written_arguments(kind, &text, t.line, &base);
                    Some(Base::Holder(args.map(|args| {
                        args.into_iter()
                            .map(|a| match a.strip_prefix('*') {
                                Some(a) if kind == Kind::Go => a.to_owned(),
                                _ => a,
                            })
                            .collect()
                    })))
                })
                .collect();
            Some((own, bases))
        })?;
        self.type_decl(kind, &sub.path, &arg)
    }
}

pub(in crate::app) enum Base<T> {
    Holder(Option<Vec<String>>),
    Other(T),
}

pub(in crate::app) fn base_argument<T: Clone>(
    sub: &T,
    (index, count): (usize, usize),
    depth: usize,
    bases: &impl Fn(&T) -> Option<(Vec<String>, Vec<Base<T>>)>,
) -> Option<(T, String)> {
    let (own, bases_of_sub) = bases(sub).filter(|_| depth < 8)?;
    let mut found = None;
    for base in bases_of_sub {
        let got = match base {
            Base::Holder(args) => {
                let args = args?;
                let arg = args.get(index).filter(|_| args.len() == count)?;
                let plain = !arg.is_empty()
                    && arg
                        .chars()
                        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '.'));
                if !plain || arg.split('.').any(|w| own.iter().any(|p| p == w)) {
                    return None;
                }
                Some((sub.clone(), arg.clone()))
            }
            Base::Other(b) => base_argument(&b, (index, count), depth + 1, bases),
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
