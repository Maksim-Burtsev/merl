use super::{CsType, Proven, Up, owner_of};
use crate::app::{App, Hit, Kind, Path, Typed, search};

impl App {
    pub(super) fn cs_up_call(&self, ty: &Typed, word: &str) -> Up {
        let mut nearest = None;
        let fits = |own: &[Hit]| {
            let fit = self.cs_fit(word, self.cs_args(), Some(""), own.to_vec());
            !fit.is_empty() && !self.cs_misfit(word, &fit)
        };
        match self.cs_climb(ty, word, 0, &fits, &mut nearest) {
            Up::Found(hits) => Up::Found(hits),
            up => nearest.map_or(up, Up::Found),
        }
    }

    pub(super) fn cs_literals(&self) -> Option<Vec<&'static str>> {
        self.definition_word(Some(Kind::CSharp))
            .and_then(|(r, _)| search::cs_literal_arguments(&self.buf.lines, self.line, r.end))
    }

    pub(super) fn cs_misfit(&self, word: &str, hits: &[Hit]) -> bool {
        let Some(args) = self.cs_literals() else {
            return false;
        };
        hits.iter().all(|h| {
            self.hit_text(h)
                .and_then(|text| search::cs_parameter_keywords(&text, h.line1, word))
                .is_some_and(|takes| {
                    args.iter()
                        .zip(&takes)
                        .any(|(a, t)| t.as_deref().is_some_and(|t| t != *a))
                })
        })
    }

    pub(super) fn cs_fit(
        &self,
        word: &str,
        args: Option<usize>,
        receiver: Option<&str>,
        hits: Vec<Hit>,
    ) -> Vec<Hit> {
        let Some(n) = args else {
            return hits;
        };
        hits.into_iter()
            .filter(|h| {
                let Some(text) = self.hit_text(h) else {
                    return true;
                };
                let Some(search::CsArity {
                    fewest: min,
                    most_unless_params: max,
                    extension_this: this,
                }) = search::cs_parameters(&text, h.line1, word)
                else {
                    return true;
                };
                let owner = match search::cs_place(&text, h.line1, word) {
                    search::CsPlace::Member { owner, .. } => owner,
                    _ => String::new(),
                };
                let by = usize::from(this && receiver.is_some_and(|q| q.is_empty() || q != owner));
                n + by >= min && max.is_none_or(|m| n + by <= m)
            })
            .collect()
    }

    pub(super) fn cs_overload(&self, word: &str, hits: Vec<Hit>) -> Vec<Hit> {
        let Some(args) = self.cs_literals() else {
            return hits;
        };
        search::literal_overloads(hits, &args, |h| {
            search::cs_parameter_keywords(&self.hit_text(h)?, h.line1, word)
        })
    }

    pub(super) fn cs_static_head(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        called: bool,
    ) -> Option<Proven> {
        if !called
            || !name.starts_with(|c: char| c.is_ascii_uppercase())
            || !search::bindings(Kind::CSharp, text, line1, name).is_empty()
            || owner_of(file, text, line1)
                .is_some_and(|o| !matches!(self.cs_up(&o, name, 0), Up::Missing { .. }))
        {
            return None;
        }
        let ty @ CsType::Project(_) = self.cs_resolve(file, text, line1, name)? else {
            return None;
        };
        Some(Proven {
            ty,
            link: name.to_owned(),
            written: name.to_owned(),
            from: (file.to_path_buf(), line1),
        })
    }
}
