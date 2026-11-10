use super::{CsType, CsWrittenType, Proven, one_more, same};
use crate::app::{App, Hit, Kind, Path, Typed, search};

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
            if !steps.whole
                || steps.new.is_some()
                || steps.awaited > 0
                || *first != (param.clone(), false)
            {
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

    pub(super) fn cs_alias_target(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        word: &str,
    ) -> Option<(usize, String)> {
        let written = |l: &str| l.contains("using") && l.contains(word) && l.contains('=');
        if !text.lines().any(written) {
            return None;
        }
        let mut aliases = search::cs_code_lines(text).filter_map(|(k, l)| {
            let u = search::cs_using(l).filter(|u| !u.fixed && u.alias.as_deref() == Some(word))?;
            Some((k + 1, u.target))
        });
        let (at, target) = aliases.next()?;
        let first_namespace = text.lines().position(search::cs_namespace_line);
        if aliases.next().is_some()
            || first_namespace.is_some_and(|n| n < at)
            || !target.contains('.')
        {
            return None;
        }
        (!self.cs_alias_hidden(here, text, line1, word, &target)).then_some((at, target))
    }

    pub(super) fn cs_alias_hidden(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        word: &str,
        target: &str,
    ) -> bool {
        let mut levels = vec![String::new()];
        let inside = search::cs_namespace(text, line1);
        let mut level = inside.as_str();
        while !level.is_empty() {
            levels.push(level.to_owned());
            level = level.rsplit_once('.').map_or("", |(outer, _)| outer);
        }
        let own = target.strip_suffix(word).and_then(|q| q.strip_suffix('.'));
        self.cs_project_types(here, word).iter().any(|h| {
            let Some(t) = self.text_of(&h.path) else {
                return true;
            };
            let ns = search::cs_namespace(&t, h.line1);
            search::cs_owner(&t, h.line1).is_some()
                || (own != Some(ns.as_str()) && levels.contains(&ns))
        })
    }

    pub(super) fn cs_project_types(&self, here: &Path, name: &str) -> Vec<Hit> {
        let pattern = search::def_patterns(Kind::CSharp, name)[..2].join("|");
        let mut hits = self.project_definitions(Kind::CSharp, here, name, &pattern);
        hits.retain(|h| {
            self.hidden_now(Kind::CSharp, h).get(h.line1 - 1) != Some(&true)
                && search::cs_type_decl(&h.text).is_some_and(|d| d.name == name)
        });
        hits
    }

    pub(super) fn cs_bases_in_project(&self, here: &Path) -> bool {
        self.cs_around().into_iter().all(|(name, line)| {
            let ty = Typed {
                name,
                path: here.to_path_buf(),
                line,
            };
            self.cs_in_project(&ty, 0)
        })
    }

    fn cs_in_project(&self, ty: &Typed, depth: usize) -> bool {
        let Some(text) = self.text_of(&ty.path) else {
            return false;
        };
        let (parts, _) = self.cs_parts(ty, &text);
        depth < 8
            && parts.iter().all(|part| {
                self.text_of(&part.path).is_some_and(|text| {
                    search::cs_bases(&text, part.line).iter().all(|base| {
                        matches!(self.cs_resolve(&part.path, &text, part.line, base),
                            Some(CsType::Project(b)) if self.cs_in_project(&b, depth + 1))
                    })
                })
            })
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

    pub(in crate::app) fn cs_new_call(&self) -> bool {
        let line = self.line_str();
        let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '@' || c == '.';
        self.definition_word(Some(Kind::CSharp))
            .is_some_and(|(r, _)| {
                line.get(..r.start)
                    .and_then(|b| b.trim_end().strip_suffix("new"))
                    .is_some_and(|b| !b.ends_with(ident))
                    && line
                        .get(r.end..)
                        .is_some_and(|a| a.trim_start().starts_with('('))
            })
    }
}
