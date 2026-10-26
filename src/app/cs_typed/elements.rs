use super::{CsType, CsWrittenType, Proven, one_more, same};
use crate::app::{App, Path, Typed, search};

impl App {
    pub(super) fn cs_await(&self, p: Proven) -> Option<Proven> {
        let written = search::cs_awaited(&p.written)?.to_owned();
        let (path, line1) = &p.from;
        let text = self.text_of(path)?;
        let ty = self.cs_resolve(path, &text, *line1, &written)?;
        let link = format!("await {}", search::cs_type_name(&written)?);
        Some(Proven {
            ty,
            link: format!("{} \u{2192} {link}", p.link),
            written,
            from: p.from,
        })
    }

    pub(super) fn cs_select(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        from: &Proven,
        arg: &str,
    ) -> Option<Proven> {
        let cut = self.truncated.get();
        let extended = !self.cs_extensions(file, "Select").is_empty();
        self.truncated.set(cut);
        if extended {
            return None;
        }
        let (param, body) = search::cs_select_lambda(arg)?;
        let e = if let search::CsValue::Type(w) = search::cs_expr(&body) {
            Proven {
                ty: self.cs_new_type(file, text, line1, &w)?,
                link: format!("{param}: {w}"),
                written: w,
                from: (file.to_path_buf(), line1),
            }
        } else {
            let steps = search::cs_steps(&body)?;
            let (first, rest) = steps.parts.split_first()?;
            if steps.new.is_some() || steps.awaited > 0 || *first != (param.clone(), false) {
                return None;
            }
            let e = self.cs_element(from, true)?;
            let mut p = Proven {
                link: format!("{param}: {}", e.written),
                ty: e.ty,
                written: e.written,
                from: e.from,
            };
            for (step, call) in rest {
                p = self.cs_step(file, &p, step, *call)?;
            }
            p
        };
        let shown = search::cs_type_name(&e.written)?;
        Some(Proven {
            ty: CsType::Outside("IEnumerable".to_owned()),
            link: format!("Select(): IEnumerable<{shown}>"),
            written: format!("IEnumerable<{}>", e.written),
            from: e.from,
        })
    }

    pub(super) fn cs_extended_step(&self, here: &Path, t: &Typed, name: &str) -> Option<Proven> {
        let cut = self.truncated.get();
        let extensions = self.cs_extensions(here, name);
        self.truncated.set(cut);
        let receiver = CsType::Project(t.clone());
        let mut found: Option<Proven> = None;
        for (hit, signature) in extensions {
            let text = self.text_of(&hit.path)?;
            let this = search::cs_extended(&signature)?;
            if search::cs_type_name(&this).as_deref() != Some(t.name.as_str()) {
                continue;
            }
            if !same(
                &self.cs_resolve(&hit.path, &text, hit.line1, &this)?,
                &receiver,
            ) {
                continue;
            }
            let w = search::cs_returns(&signature, name)?;
            let ty = self.cs_resolve(&hit.path, &text, hit.line1, &w)?;
            let link = format!("{name}(): {}", search::cs_type_name(&w)?);
            let next = Proven {
                ty,
                link,
                written: w,
                from: (hit.path.clone(), hit.line1),
            };
            one_more(&mut found, next)?;
        }
        found
    }

    pub(super) fn cs_element(&self, of: &Proven, collection: bool) -> Option<CsWrittenType> {
        let CsType::Outside(outer) = &of.ty else {
            return None;
        };
        if collection && !search::cs_collection(outer) {
            return None;
        }
        let written = search::cs_one_argument(&of.written)?;
        let (path, line1) = &of.from;
        let text = self.text_of(path)?;
        Some(CsWrittenType {
            ty: self.cs_resolve(path, &text, *line1, &written)?,
            written,
            from: of.from.clone(),
        })
    }
}
