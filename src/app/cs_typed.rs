use std::cell::OnceCell;

use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, Regex, Typed, search};

mod elements;

#[derive(Debug, Clone)]
enum CsType {
    Project(Typed),
    Outside(String),
}

pub(super) enum CsAnswer {
    Found(Vec<Candidate>),
    Outside { links: String },
}

enum Up {
    Found(Vec<Hit>),
    Outside { walked: Vec<String> },
    Missing { walked: Vec<String> },
    Unknown,
}

struct Proven {
    ty: CsType,
    link: String,
    written: String,
    from: (PathBuf, usize),
}

struct CsWrittenType {
    ty: CsType,
    written: String,
    from: (PathBuf, usize),
}

impl App {
    pub(super) fn cs_typed(
        &self,
        here: &Path,
        word: &str,
        chain: &[String],
    ) -> Result<CsAnswer, String> {
        let text = self.buf.lines.join("\n");
        let (first, fields) = chain.split_first().ok_or_else(|| word.to_owned())?;
        let Proven { mut ty, link, .. } = self
            .cs_name_type(here, &text, self.line + 1, first, 1)
            .ok_or_else(|| first.clone())?;
        let mut links = vec![link];
        for (i, field) in fields.iter().enumerate() {
            if i == 5 {
                return Err(field.clone());
            }
            let CsType::Project(t) = &ty else {
                return Err(field.clone());
            };
            let Proven { ty: next, link, .. } =
                self.cs_field_type(t, field).ok_or_else(|| field.clone())?;
            links.push(link);
            ty = next;
        }
        let label = links.join(" \u{2192} ");
        self.cs_member_answer(here, &ty, word, &label)
            .ok_or_else(|| chain.last().cloned().unwrap_or_default())
    }

    pub(super) fn cs_initializer(
        &self,
        here: &Path,
        word: &str,
        range: &std::ops::Range<usize>,
    ) -> Option<CsAnswer> {
        let written = search::cs_initialized(&self.buf.lines, self.line, range.start, range.end)?;
        let text = self.buf.lines.join("\n");
        let ty = self.cs_resolve(here, &text, self.line + 1, &written)?;
        let label = format!("new {}", search::cs_type_name(&written)?);
        match ty {
            CsType::Outside(_) => Some(CsAnswer::Outside { links: label }),
            CsType::Project(t) => match self.cs_up(&t, word, 0) {
                Up::Found(hits) => Some(CsAnswer::Found(receivers(hits, &label))),
                Up::Outside { .. } => Some(CsAnswer::Outside { links: label }),
                Up::Missing { .. } | Up::Unknown => None,
            },
        }
    }

    pub(super) fn cs_constructed(&self, here: &Path, word: &str, before: &str) -> Option<CsAnswer> {
        static NEW: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?:^|[^\w@.])(new\s)").expect("the regex is a literal")
        });
        let before = before.trim_end().strip_suffix('.')?;
        let at = NEW.captures_iter(before).last()?.get(1)?.start();
        let made = before.get(at..).filter(|m| !m.contains('{'))?;
        let search::CsValue::Type(written) = search::cs_expr(made) else {
            return None;
        };
        let text = self.buf.lines.join("\n");
        let ty = self.cs_new_type(here, &text, self.line + 1, &written)?;
        let label = format!("new {}", search::cs_type_name(&written)?);
        self.cs_member_answer(here, &ty, word, &label)
    }

    fn cs_new_type(&self, file: &Path, text: &str, line1: usize, written: &str) -> Option<CsType> {
        let ty = self.cs_resolve(file, text, line1, written)?;
        if let CsType::Project(t) = &ty
            && written.contains('.')
        {
            let hit = Hit {
                path: t.path.clone(),
                line1: t.line,
                byte_col: None,
                text: String::new(),
                deleted: None,
            };
            (self.cs_namespace_hits(text, line1, written, &[&hit]))
                .filter(|picked| !picked.is_empty())?;
        }
        Some(ty)
    }

    pub(super) fn cs_call_result(&self, here: &Path, word: &str, start: usize) -> Option<CsAnswer> {
        let receiver = search::cs_receiver(&self.buf.lines, self.line, start)?;
        let text = self.buf.lines.join("\n");
        let p = self.cs_chain_type(here, &text, self.line + 1, &receiver, 1)?;
        self.cs_member_answer(here, &p.ty, word, &p.link)
    }

    fn cs_chain_type(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        expr: &str,
        hops: usize,
    ) -> Option<Proven> {
        let coalesced = || {
            let (left, right) = search::cs_coalesced(expr)?;
            let l = self.cs_chain_type(file, text, line1, left, hops)?;
            let r = self.cs_chain_type(file, text, line1, right, hops)?;
            let (a, b) = (self.cs_element(&l, true)?, self.cs_element(&r, true)?);
            same(&a.ty, &b.ty).then_some(l)
        };
        if let Some(l) = coalesced() {
            return Some(l);
        }
        if let Some((class, written)) = search::cs_framework_value(expr) {
            let CsType::Outside(_) = self.cs_resolve(file, text, line1, &class)? else {
                return None;
            };
            let ty = match written.strip_suffix("[]") {
                Some(_) => CsType::Outside("Array".to_owned()),
                None => self.cs_resolve(file, text, line1, &written)?,
            };
            return Some(Proven {
                ty,
                link: format!("{class}: {written}"),
                written,
                from: (file.to_path_buf(), line1),
            });
        }
        let search::CsSteps {
            new,
            parts,
            args,
            awaited,
            ..
        } = search::cs_steps(expr)?;
        let here = (file.to_path_buf(), line1);
        let (mut p, rest) = match (new, parts.split_first()) {
            (Some(w), _) => {
                let link = format!("new {}", search::cs_type_name(&w)?);
                let alias = (text.lines().filter_map(search::cs_using))
                    .find(|u| !u.fixed && u.alias.as_deref() == Some(w.as_str()));
                let w = alias.map_or(w, |u| u.target);
                let ty = self.cs_new_type(file, text, line1, &w)?;
                let p = Proven {
                    ty,
                    link,
                    written: w,
                    from: here,
                };
                (p, parts.as_slice())
            }
            (None, Some(((name, false), rest))) => {
                (self.cs_name_type(file, text, line1, name, hops)?, rest)
            }
            (None, Some(((name, true), rest))) => {
                let w = self.cs_call_type(file, text, line1, name, false, hops)?;
                let link = format!("{name}(): {}", search::cs_type_name(&w.written)?);
                let p = Proven {
                    ty: w.ty,
                    link,
                    written: w.written,
                    from: w.from,
                };
                (p, rest)
            }
            _ => return None,
        };
        if awaited == 1 {
            p = self.cs_await(p)?;
        }
        let mut links = vec![p.link.clone()];
        let skipped = parts.len() - rest.len();
        for (i, (name, call)) in rest.iter().enumerate() {
            let arg = args.get(skipped + i).map_or("", String::as_str);
            p = match &p.ty {
                CsType::Outside(outer)
                    if *call
                        && name == "Select"
                        && search::cs_collection(outer)
                        && generic(&p.written) =>
                {
                    self.cs_select(file, text, line1, &p, arg)?
                }
                _ => self.cs_step(file, &p, name, *call)?,
            };
            if skipped + i + 1 == awaited {
                p = self.cs_await(p)?;
            }
            links.push(p.link.clone());
        }
        p.link = links.join(" \u{2192} ");
        Some(p)
    }

    fn cs_step(&self, here: &Path, from: &Proven, name: &str, call: bool) -> Option<Proven> {
        let t = match &from.ty {
            CsType::Project(t) if !call => return self.cs_field_type(t, name),
            CsType::Project(t) => t,
            CsType::Outside(outer) if call && generic(&from.written) => {
                let linq = search::cs_linq(name).filter(|_| search::cs_collection(outer))?;
                let cut = self.truncated.get();
                let unextended = self.cs_extensions(here, name).is_empty();
                self.truncated.set(cut);
                if !unextended {
                    return None;
                }
                return match linq {
                    search::CsLinq::Same => Some(Proven {
                        ty: from.ty.clone(),
                        link: format!("{name}()"),
                        written: from.written.clone(),
                        from: from.from.clone(),
                    }),
                    search::CsLinq::Element => {
                        let e = self.cs_element(from, true)?;
                        let link = format!("{name}(): {}", search::cs_type_name(&e.written)?);
                        Some(Proven {
                            ty: e.ty,
                            link,
                            written: e.written,
                            from: e.from,
                        })
                    }
                };
            }
            CsType::Outside(_) if !call || generic(&from.written) => return None,
            CsType::Outside(_) => {
                let cut = self.truncated.get();
                let unextended = self.cs_extensions(here, name).is_empty();
                self.truncated.set(cut);
                return unextended.then(|| Proven {
                    ty: CsType::Outside(format!("{name}()")),
                    link: format!("{name}()"),
                    written: format!("{name}()"),
                    from: from.from.clone(),
                });
            }
        };
        let hits = match self.cs_up(t, name, 0) {
            Up::Found(hits) => hits,
            Up::Missing { .. } => return self.cs_extended_step(here, t, name),
            Up::Outside { .. } | Up::Unknown => return None,
        };
        let mut found: Option<Proven> = None;
        for hit in hits {
            let w = search::cs_returns(&hit.text, name)?;
            let at = self.text_of(&hit.path)?;
            let ty = self.cs_resolve(&hit.path, &at, hit.line1, &w)?;
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

    fn cs_member_answer(
        &self,
        here: &Path,
        ty: &CsType,
        word: &str,
        label: &str,
    ) -> Option<CsAnswer> {
        let (walked, outside, extends) = match ty {
            CsType::Outside(name) => (vec![name.clone()], true, true),
            CsType::Project(t) => match self.cs_up(t, word, 0) {
                Up::Found(hits) => {
                    let fit = self.cs_fit(word, self.cs_args(), Some(""), hits.clone());
                    let hits = if fit.is_empty() {
                        hits
                    } else {
                        self.cs_overload(word, fit)
                    };
                    return Some(CsAnswer::Found(receivers(hits, label)));
                }
                Up::Outside { walked } => (walked, true, false),
                Up::Missing { walked } => (walked, false, true),
                Up::Unknown => return None,
            },
        };
        let extensions = self.cs_extensions(here, word);
        self.truncated.set(false);
        let (of, other): (Vec<_>, Vec<_>) = extensions.into_iter().partition(|(_, signature)| {
            search::cs_extended(signature)
                .and_then(|t| search::cs_type_name(&t))
                .is_some_and(|t| walked.contains(&t))
        });
        let of: Vec<Hit> = of.into_iter().map(|(h, _)| h).collect();
        match (of.is_empty() || !extends, other.is_empty()) {
            (false, _) => Some(CsAnswer::Found(receivers(of, label))),
            (true, true) if outside => Some(CsAnswer::Outside {
                links: label.to_owned(),
            }),
            _ => None,
        }
    }

    fn cs_extensions(&self, here: &Path, name: &str) -> Vec<(Hit, String)> {
        let n = regex::escape(name);
        let pattern = format!(r"\bstatic\b.*\b{n}\s*(?:<[^>]*>)?\s*\(\s*(?:this\s|$)");
        self.project_grep(Kind::CSharp, here, &pattern)
            .into_iter()
            .filter_map(|h| {
                let line = h.text.trim_end();
                if !line.ends_with('(') {
                    let signature = line.to_owned();
                    return Some((h, signature));
                }
                let text = self.text_of(&h.path)?;
                let next = text.lines().skip(h.line1).find(|l| !l.trim().is_empty())?;
                let next = next.trim();
                let signature = format!("{line}{next}");
                let this = next.strip_prefix("this")?;
                this.starts_with(char::is_whitespace)
                    .then_some((h, signature))
            })
            .collect()
    }

    fn cs_up(&self, ty: &Typed, word: &str, depth: usize) -> Up {
        let mut walked = vec![ty.name.clone()];
        let Some(text) = self.text_of(&ty.path) else {
            return Up::Outside { walked };
        };
        let (parts, partial) = self.cs_parts(ty, &text);
        let around = OnceCell::new();
        let own: Vec<Hit> = parts
            .iter()
            .flat_map(|p| {
                let text = self.text_of(&p.path).unwrap_or_default();
                let around = &around;
                self.cs_members(p, word)
                    .into_iter()
                    .filter(move |h| self.cs_sees(&search::cs_place(&text, h.line1, word), around))
            })
            .collect();
        if !own.is_empty() {
            return Up::Found(own);
        }
        if depth >= 8 {
            return Up::Outside { walked };
        }
        let mut outside = false;
        for part in &parts {
            let Some(text) = self.text_of(&part.path) else {
                return Up::Unknown;
            };
            for base in search::cs_bases(&text, part.line) {
                match self.cs_resolve(&part.path, &text, part.line, &base) {
                    Some(CsType::Project(b)) => match self.cs_up(&b, word, depth + 1) {
                        Up::Found(hits) => return Up::Found(hits),
                        Up::Outside { walked: more } => {
                            outside = true;
                            walked.extend(more);
                        }
                        Up::Missing { walked: more } => walked.extend(more),
                        Up::Unknown => return Up::Unknown,
                    },
                    Some(CsType::Outside(name)) => {
                        outside = true;
                        walked.push(name);
                    }
                    None => return Up::Unknown,
                }
            }
        }
        match (partial, outside) {
            (true, _) => Up::Unknown,
            (false, true) => Up::Outside { walked },
            (false, false) => Up::Missing { walked },
        }
    }

    fn cs_around(&self) -> Vec<(String, usize)> {
        let text = self.buf.lines.join("\n");
        let mut around = Vec::new();
        let mut at = search::cs_owner(&text, self.line + 1);
        while let Some(decl) = at.filter(|_| around.len() < 8) {
            let Some(d) = self
                .buf
                .lines
                .get(decl - 1)
                .and_then(|l| search::cs_type_decl(l))
            else {
                break;
            };
            around.push((d.name, decl));
            at = search::cs_owner(&text, decl);
        }
        around
    }

    fn cs_sees(&self, place: &search::CsPlace, around: &OnceCell<Vec<String>>) -> bool {
        let search::CsPlace::Member {
            owner,
            private: true,
        } = place
        else {
            return true;
        };
        around
            .get_or_init(|| self.cs_around().into_iter().map(|(name, _)| name).collect())
            .contains(owner)
    }

    #[expect(
        clippy::suspicious_operation_groupings,
        reason = "a hit's `line1` and a type's `line` are the same row, named apart"
    )]
    fn cs_parts(&self, ty: &Typed, text: &str) -> (Vec<Typed>, bool) {
        let mut parts = vec![ty.clone()];
        let partial = Regex::new(r"\bpartial\b").is_ok_and(|re| {
            text.lines()
                .nth(ty.line - 1)
                .is_some_and(|l| re.is_match(l))
        });
        if partial {
            let n = regex::escape(&ty.name);
            let pattern =
                format!(r"\bpartial\s+(?:record\s+)?(?:class|struct|interface|record)\s+{n}\b");
            let cut = self.truncated.get();
            let namespace = search::cs_namespace(text, ty.line);
            for h in self.project_grep(Kind::CSharp, &ty.path, &pattern) {
                let same = self
                    .text_of(&h.path)
                    .is_some_and(|t| search::cs_namespace(&t, h.line1) == namespace);
                if same && (h.path != ty.path || h.line1 != ty.line) {
                    parts.push(Typed {
                        name: ty.name.clone(),
                        path: h.path,
                        line: h.line1,
                    });
                }
            }
            self.truncated.set(cut);
        }
        (parts, partial)
    }

    fn cs_members(&self, ty: &Typed, word: &str) -> Vec<Hit> {
        let Some(text) = self.text_of(&ty.path) else {
            return Vec::new();
        };
        if word == ty.name {
            return Vec::new();
        }
        let Ok(re) = Regex::new(&search::def_patterns(Kind::CSharp, word).join("|")) else {
            return Vec::new();
        };
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::CSharp, &text);
        let hit = |line: usize| Hit {
            path: ty.path.clone(),
            line1: line,
            byte_col: None,
            text: lines[line - 1].to_owned(),
            deleted: None,
        };
        let mut hits: Vec<Hit> = lines
            .iter()
            .enumerate()
            .filter(|&(i, l)| {
                literal.get(i) != Some(&true)
                    && re.is_match(l)
                    && search::cs_owner(&text, i + 1) == Some(ty.line)
                    && !matches!(
                        search::cs_place(&text, i + 1, word),
                        search::CsPlace::Local { .. }
                    )
            })
            .map(|(i, _)| hit(i + 1))
            .collect();
        let header = lines.get(ty.line - 1).copied().unwrap_or_default();
        let positional = Regex::new(&format!(
            r"\brecord\b.*[(,]\s*(?:\[[^\]]*\]\s*)*[\w.<>\[\]?, ]+\s@?{}\s*[,)=]",
            regex::escape(word)
        ));
        if hits.is_empty() && positional.is_ok_and(|re| re.is_match(header)) {
            hits.push(hit(ty.line));
        }
        hits
    }

    fn cs_field_type(&self, ty: &Typed, field: &str) -> Option<Proven> {
        let Up::Found(hits) = self.cs_up(ty, field, 0) else {
            return None;
        };
        let [hit] = hits.as_slice() else {
            return None;
        };
        let search::CsValue::Type(written) = search::cs_declared(&hit.text, field) else {
            return None;
        };
        let text = self.text_of(&hit.path)?;
        let ty = self.cs_resolve(&hit.path, &text, hit.line1, &written)?;
        let link = format!("{field}: {}", search::cs_type_name(&written)?);
        Some(Proven {
            ty,
            link,
            written,
            from: (hit.path.clone(), hit.line1),
        })
    }

    fn cs_name_type(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        hops: usize,
    ) -> Option<Proven> {
        if matches!(name, "this" | "base") {
            return self.cs_this(file, text, line1, name);
        }
        let lines: Vec<&str> = text.lines().collect();
        let bindings = search::bindings(Kind::CSharp, text, line1, name);
        if bindings.is_empty() {
            return self.cs_field_type(&owner_of(file, text, line1)?, name);
        }
        if bindings.first().is_some_and(|b| b.line1 == line1)
            && let Some(w) = search::cs_lambda_delegate(lines.get(line1 - 1)?, name)
        {
            let ty = self.cs_resolve(file, text, line1, &w)?;
            let link = format!("{name}: {}", search::cs_type_name(&w)?);
            return Some(Proven {
                ty,
                link,
                written: w,
                from: (file.to_path_buf(), line1),
            });
        }
        let mut found: Option<Proven> = None;
        for b in bindings {
            let at = search::cs_written_line(&lines, b.line1, name);
            let value = search::cs_declared(lines.get(at - 1)?, name);
            let deep = matches!(&value, search::CsValue::Call { callee, awaited: false }
                if callee.matches('.').count() > 1);
            let chain = search::cs_var_chain(&lines, at - 1, name)
                .filter(|(_, continued)| *continued || deep || value == search::CsValue::Unknown);
            let here = (file.to_path_buf(), at);
            let spent = if at < line1 {
                hops
            } else {
                hops.saturating_sub(1)
            };
            let CsWrittenType { ty, written, from } = match (chain, value) {
                (Some((expr, _)), _) if hops > 0 => {
                    let p = self.cs_chain_type(file, text, at, &expr, hops - 1)?;
                    CsWrittenType {
                        ty: p.ty,
                        written: p.written,
                        from: p.from,
                    }
                }
                (None, search::CsValue::Type(w)) => CsWrittenType {
                    ty: self.cs_resolve(file, text, at, &w)?,
                    written: w,
                    from: here,
                },
                (None, search::CsValue::Call { callee, awaited }) if hops > 0 => {
                    self.cs_call_type(file, text, at, &callee, awaited, hops - 1)?
                }
                (None, search::CsValue::Element(items)) if hops > 0 => {
                    let p = self.cs_name_type(file, text, at, &items, spent)?;
                    self.cs_element(&p, false)?
                }
                (None, search::CsValue::Indexed(items)) if hops > 0 => {
                    let p = self.cs_name_type(file, text, at, &items, spent)?;
                    self.cs_element(&p, true)?
                }
                (None, search::CsValue::Unknown)
                    if lines
                        .get(at - 1)
                        .is_some_and(|l| search::cs_rule_lambda(l, name)) =>
                {
                    self.cs_validated(file, text, at)?
                }
                (None, search::CsValue::Unknown) if hops > 0 => {
                    let receiver = search::cs_lambda_call(&lines, at - 1, name)?;
                    let p = self.cs_chain_type(file, text, at, &receiver, spent)?;
                    self.cs_element(&p, true)?
                }
                _ => return None,
            };
            let shown = search::cs_type_name(&written).unwrap_or_else(|| written.clone());
            let link = format!("{name}: {shown}");
            let next = Proven {
                ty,
                link,
                written,
                from,
            };
            one_more(&mut found, next)?;
        }
        found
    }

    fn cs_validated(&self, file: &Path, text: &str, line1: usize) -> Option<CsWrittenType> {
        let owner = owner_of(file, text, line1)?;
        let [base] = search::cs_bases(text, owner.line).try_into().ok()?;
        let written = search::cs_validated(&base)?;
        let CsType::Outside(_) = self.cs_resolve(file, text, owner.line, &base)? else {
            return None;
        };
        Some(CsWrittenType {
            ty: self.cs_resolve(file, text, owner.line, &written)?,
            written,
            from: (file.to_path_buf(), owner.line),
        })
    }

    fn cs_this(&self, file: &Path, text: &str, line1: usize, name: &str) -> Option<Proven> {
        let t = owner_of(file, text, line1)?;
        if name == "this" {
            let link = format!("this: {}", t.name);
            return Some(Proven {
                written: t.name.clone(),
                from: (file.to_path_buf(), t.line),
                ty: CsType::Project(t),
                link,
            });
        }
        let base = search::cs_bases(text, t.line).into_iter().next()?;
        let ty = self.cs_resolve(file, text, t.line, &base)?;
        let link = format!("base: {}", search::cs_type_name(&base)?);
        Some(Proven {
            ty,
            link,
            written: base,
            from: (file.to_path_buf(), t.line),
        })
    }

    fn cs_call_type(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        callee: &str,
        awaited: bool,
        hops: usize,
    ) -> Option<CsWrittenType> {
        let parts: Vec<&str> = callee.split('.').collect();
        let (method, receiver) = parts.split_last()?;
        let owner = match receiver {
            [] => owner_of(file, text, line1)?,
            [one] => match self.cs_name_type(file, text, line1, one, hops) {
                Some(Proven {
                    ty: CsType::Project(t),
                    ..
                }) => t,
                Some(Proven {
                    ty: CsType::Outside(_),
                    ..
                }) => return None,
                None => match self.cs_resolve(file, text, line1, one)? {
                    CsType::Project(t) => t,
                    CsType::Outside(_) => return None,
                },
            },
            _ => return None,
        };
        let Up::Found(hits) = self.cs_up(&owner, method, 0) else {
            return None;
        };
        let [hit] = hits.as_slice() else {
            return None;
        };
        let written = search::cs_returns(&hit.text, method)?;
        let written = if awaited {
            search::cs_awaited(&written)?.to_owned()
        } else {
            written
        };
        let at = self.text_of(&hit.path)?;
        let ty = self.cs_resolve(&hit.path, &at, hit.line1, &written)?;
        Some(CsWrittenType {
            ty,
            written,
            from: (hit.path.clone(), hit.line1),
        })
    }

    fn cs_resolve(&self, file: &Path, text: &str, line1: usize, written: &str) -> Option<CsType> {
        let name = search::cs_type_name(written)?;
        if search::cs_generic_param(text, line1, &name) {
            return None;
        }
        let cut = self.truncated.get();
        let pattern = search::def_patterns(Kind::CSharp, &name)[..4].join("|");
        let mut hits = self.project_definitions(Kind::CSharp, file, &name, &pattern);
        hits.retain(|h| self.hidden_now(Kind::CSharp, h).get(h.line1 - 1) != Some(&true));
        let (types, other): (Vec<&Hit>, Vec<&Hit>) = hits
            .iter()
            .partition(|h| search::cs_type_decl(&h.text).is_some_and(|d| d.name == name));
        let found = match (types.as_slice(), other.is_empty()) {
            ([one], true) => Some(CsType::Project(Typed {
                name: name.clone(),
                path: one.path.clone(),
                line: one.line1,
            })),
            ([], true) => Some(CsType::Outside(name)),
            (many, true) => self.cs_one_of(text, line1, written, &name, many),
            _ => None,
        };
        self.truncated.set(cut);
        found
    }

    fn cs_one_of(
        &self,
        text: &str,
        line1: usize,
        written: &str,
        name: &str,
        many: &[&Hit],
    ) -> Option<CsType> {
        let picked: Vec<&Hit> = (self.cs_namespace_hits(text, line1, written, many)?)
            .into_iter()
            .map(|i| many[i])
            .collect();
        let shape = partial_shape(name);
        let shapes: Vec<Option<String>> = picked
            .iter()
            .map(|h| part_shape(&shape, &h.text).map(|(generics, _)| generics))
            .collect();
        let one = match (picked.as_slice(), shapes.first()) {
            ([one], _) => one,
            ([first, ..], Some(Some(s))) if shapes.iter().all(|t| t.as_ref() == Some(s)) => first,
            _ => return None,
        };
        Some(CsType::Project(Typed {
            name: name.to_owned(),
            path: one.path.clone(),
            line: one.line1,
        }))
    }

    fn cs_namespace_hits(
        &self,
        text: &str,
        line1: usize,
        written: &str,
        many: &[&Hit],
    ) -> Option<Vec<usize>> {
        let mut namespaces = Vec::new();
        for h in many {
            let t = self.text_of(&h.path)?;
            if search::cs_owner(&t, h.line1).is_some() {
                return None;
            }
            namespaces.push(search::cs_namespace(&t, h.line1));
        }
        Some(search::cs_namespace_pick(text, line1, written, &namespaces))
    }

    fn cs_type_namesakes(
        &self,
        here: &Path,
        word: &str,
        chain: Option<&[String]>,
        hits: Vec<Hit>,
    ) -> Vec<Hit> {
        let types = hits.len() > 1
            && hits.iter().all(|h| {
                h.deleted.is_none()
                    && (h.path != here || h.line1 != self.line + 1)
                    && search::cs_type_decl(&h.text).is_some_and(|d| d.name == word)
            });
        if !types {
            return hits;
        }
        let (hidden, mut live): (Vec<Hit>, Vec<Hit>) = hits
            .into_iter()
            .partition(|h| self.hidden_now(Kind::CSharp, h).get(h.line1 - 1) == Some(&true));
        let written = match chain {
            Some(c) if !c.is_empty() => format!("{}.{word}", c.join(".")),
            _ => word.to_owned(),
        };
        let text = self.buf.lines.join("\n");
        let plain = one_namespace(&text, None)
            && live.iter().all(|h| {
                self.text_of(&h.path)
                    .is_some_and(|t| one_namespace(&t, Some(h.line1)))
            });
        let picked = plain
            .then(|| {
                let all: Vec<&Hit> = live.iter().collect();
                self.cs_namespace_hits(&text, self.line + 1, &written, &all)
            })
            .flatten()
            .unwrap_or_default();
        if !picked.is_empty() {
            live = (live.into_iter().enumerate())
                .filter_map(|(i, h)| picked.contains(&i).then_some(h))
                .collect();
        }
        if chain.is_none() && self.cs_new_call() {
            live = self.cs_partial_constructors(word, live);
        }
        live.extend(hidden);
        live
    }

    fn cs_new_call(&self) -> bool {
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

    fn cs_partial_constructors(&self, word: &str, parts: Vec<Hit>) -> Vec<Hit> {
        let texts: Option<Vec<String>> = parts.iter().map(|h| self.text_of(&h.path)).collect();
        let Some(texts) = texts.filter(|_| parts.len() > 1) else {
            return parts;
        };
        let mut namespaces =
            (parts.iter().zip(&texts)).map(|(h, t)| search::cs_namespace(t, h.line1));
        let first = namespaces.next();
        let shape = partial_shape(word);
        let mut shapes = parts.iter().map(|h| part_shape(&shape, &h.text));
        let head = shapes.next().flatten();
        let one_type = namespaces.all(|n| Some(n) == first)
            && head.as_ref().is_some_and(|(_, primary)| !primary)
            && shapes.all(|s| s == head);
        if !one_type {
            return parts;
        }
        let ctor = Regex::new(&format!(
            r"^\s*(?:\[[^\]]*\]\s*)*((?:\w+\s+)*)@?{}\s*\(",
            regex::escape(word)
        ))
        .expect("an escaped name keeps the pattern valid");
        let instance = |modifiers: &str| {
            modifiers.split_whitespace().all(|m| {
                matches!(
                    m,
                    "public" | "private" | "protected" | "internal" | "extern" | "unsafe"
                )
            })
        };
        let mut ctors = Vec::new();
        for (h, text) in parts.iter().zip(&texts) {
            let literal = search::literal_lines(Kind::CSharp, text);
            for (i, l) in text.lines().enumerate() {
                if literal.get(i) != Some(&true)
                    && ctor.captures(l).is_some_and(|c| instance(&c[1]))
                    && search::cs_owner(text, i + 1) == Some(h.line1)
                {
                    ctors.push(Hit {
                        path: h.path.clone(),
                        line1: i + 1,
                        byte_col: None,
                        text: l.to_owned(),
                        deleted: None,
                    });
                }
            }
        }
        let ctors = self.cs_fit(word, self.cs_args(), None, ctors);
        if ctors.is_empty() { parts } else { ctors }
    }
}

impl App {
    pub(super) fn cs_sight(&self, kind: Kind, here: &Path) -> Option<search::CsProjects> {
        if kind != Kind::CSharp {
            return None;
        }
        search::cs_projects(&self.files, here, |p| {
            std::fs::read_to_string(self.root.join(p)).ok()
        })
    }

    pub(super) fn cs_declares(&self, here: &Path, name: &str) -> bool {
        let cut = self.truncated.get();
        let n = regex::escape(name);
        let pattern = search::def_patterns(Kind::CSharp, name).join("|");
        let declared = !self
            .project_definitions(Kind::CSharp, here, name, &pattern)
            .is_empty()
            || !self
                .project_grep(
                    Kind::CSharp,
                    here,
                    &format!(r"^\s*namespace\s+(?:[\w.]+\.)?{n}\b"),
                )
                .is_empty();
        self.truncated.set(cut);
        declared
    }

    pub(super) fn cs_enum_member(
        &self,
        here: &Path,
        text: &str,
        chain: &[String],
        word: &str,
        path: &str,
    ) -> Option<Vec<Candidate>> {
        let kind = Kind::CSharp;
        let owner = &chain[chain.len() - 1];
        let owners = search::def_patterns(kind, owner).join("|");
        let members: Vec<(Candidate, String)> = self
            .project_definitions(kind, here, owner, &owners)
            .into_iter()
            .filter_map(|h| {
                let text = self.text_of(&h.path)?;
                let (line, col) = search::enum_member(kind, &text, h.line1, word)?;
                let hit = Hit {
                    deleted: None,
                    text: text.lines().nth(line - 1)?.to_owned(),
                    path: h.path,
                    line1: line,
                    byte_col: Some(col),
                };
                let reason = Reason::Path(path.to_owned());
                Some((Candidate { hit, reason }, search::cs_namespace(&text, line)))
            })
            .collect();
        let prefix = chain[..chain.len() - 1].join(".");
        let inside = search::cs_namespace(text, self.line + 1);
        let mut opened = search::cs_usings(text);
        if !members.is_empty() {
            let project = search::cs_own_project(&self.files, here).unwrap_or_default();
            let global = r#"^\u{feff}?\s*global\s+using\s+[\w.]+\s*;|<Using\s+Include=""#;
            let files = |p: &Path| {
                p.starts_with(&project) && p.extension().is_some_and(|e| e == "cs" || e == "csproj")
            };
            for h in self.grep(global, false, false, files).unwrap_or_default() {
                opened.extend(search::cs_usings(&h.text));
            }
        }
        let sees = |ns: &String| match prefix.as_str() {
            "" => {
                ns.is_empty()
                    || inside == *ns
                    || inside.starts_with(&format!("{ns}."))
                    || opened.contains(ns)
            }
            p => ns == p || ns.ends_with(&format!(".{p}")),
        };
        let named: Vec<Candidate> = members
            .into_iter()
            .filter(|(_, ns)| sees(ns))
            .map(|(c, _)| c)
            .collect();
        if named.is_empty() {
            self.truncated.set(false);
            return None;
        }
        Some(named)
    }

    pub(super) fn cs_aliased(
        &self,
        here: &Path,
        word: &str,
        range: &std::ops::Range<usize>,
    ) -> Option<Vec<Candidate>> {
        let line = self.line_str();
        let typed = search::cs_type_position(line, range.start, range.end)
            && !search::cs_constant_may_stand(line, range.start, range.end)
            && !line.get(range.end..)?.trim_start().starts_with(':');
        let text = self.buf.lines.join("\n");
        if !typed || search::cs_generic_param(&text, self.line + 1, word) {
            return None;
        }
        let mut aliases = text.lines().enumerate().filter_map(|(k, l)| {
            let u = search::cs_using(l).filter(|u| !u.fixed && u.alias.as_deref() == Some(word))?;
            Some((k + 1, u.target))
        });
        let (at, target) = aliases.next()?;
        if !self.cs_bases_in_project(here) {
            return None;
        }
        let first_namespace = text.lines().position(search::cs_namespace_line);
        if aliases.next().is_some()
            || first_namespace.is_some_and(|n| n < at)
            || !target.contains('.')
            || search::cs_type_name(&target).as_deref() != Some(word)
        {
            return None;
        }
        let pattern = search::def_patterns(Kind::CSharp, word)[..2].join("|");
        let mut hits = self.project_definitions(Kind::CSharp, here, word, &pattern);
        hits.retain(|h| self.hidden_now(Kind::CSharp, h).get(h.line1 - 1) != Some(&true));
        let types: Vec<&Hit> = (hits.iter())
            .filter(|h| search::cs_type_decl(&h.text).is_some_and(|d| d.name == word))
            .collect();
        let [one] = self.cs_namespace_hits(&text, at, &target, &types)?[..] else {
            return None;
        };
        let mut levels = vec![String::new()];
        let inside = search::cs_namespace(&text, self.line + 1);
        let mut level = inside.as_str();
        while !level.is_empty() {
            levels.push(level.to_owned());
            level = level.rsplit_once('.').map_or("", |(outer, _)| outer);
        }
        for (i, h) in types.iter().enumerate() {
            let other = self.text_of(&h.path)?;
            if i != one && levels.contains(&search::cs_namespace(&other, h.line1)) {
                return None;
            }
        }
        Some(vec![Candidate {
            hit: types[one].clone(),
            reason: Reason::Path(format!("using {word} = {target}")),
        }])
    }

    pub(super) fn cs_namespace_segment(
        &self,
        here: &Path,
        range: &std::ops::Range<usize>,
    ) -> Option<Vec<Candidate>> {
        let search::CsNamespacePrefix { prefix, certain } =
            search::cs_namespace_prefix(self.line_str(), range.start, range.end)?;
        let p = regex::escape(&prefix);
        let pattern = format!(r"^\u{{feff}}?\s*namespace\s+{p}(?:\.[\w.]+)?\s*[;{{]?\s*$");
        let found: Vec<Candidate> = self
            .project_grep(Kind::CSharp, here, &pattern)
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect();
        (certain || !found.is_empty()).then_some(found)
    }

    pub(super) fn cs_types_only(
        &self,
        here: &Path,
        word: &str,
        range: std::ops::Range<usize>,
    ) -> bool {
        let line = self.line_str();
        if !search::cs_type_position(line, range.start, range.end) {
            return false;
        }
        if !search::cs_constant_may_stand(line, range.start, range.end) {
            return true;
        }
        let cut = self.truncated.get();
        let found = match self.cs_class_first(here, word, true) {
            Ok(found) => !found.is_empty(),
            Err(walked) => {
                let types = search::cs_type_patterns(word).join("|");
                let hits = self.project_definitions(Kind::CSharp, here, word, &types);
                !self
                    .cs_reachable(here, word, None, walked.as_deref(), hits)
                    .is_empty()
            }
        };
        self.truncated.set(cut);
        found
    }

    pub(super) fn cs_class_first(
        &self,
        here: &Path,
        word: &str,
        types_only: bool,
    ) -> Result<Vec<Candidate>, Option<Vec<String>>> {
        let line = self.line_str();
        let (r, _) = self.definition_word(Some(Kind::CSharp)).ok_or(None)?;
        let (before, after) = (
            line.get(..r.start).ok_or(None)?.trim_end(),
            line.get(r.end..).ok_or(None)?.trim_start(),
        );
        if after.starts_with('=')
            && !after.starts_with("==")
            && !after.starts_with("=>")
            && (before.ends_with(['{', ','])
                || (before.is_empty() && !line.trim_end().ends_with(';')))
        {
            return Err(None);
        }
        let around: Vec<Typed> = (self.cs_around().into_iter())
            .map(|(name, line)| Typed {
                name,
                path: here.to_path_buf(),
                line,
            })
            .collect();
        if around.is_empty() {
            return Err(None);
        }
        let level = Level {
            around: OnceCell::from(around.iter().map(|t| t.name.clone()).collect::<Vec<_>>()),
            types: types_only.then(|| {
                Regex::new(&search::def_patterns(Kind::CSharp, word)[..2].join("|"))
                    .expect("an escaped name keeps the pattern valid")
            }),
            args: self.cs_args(),
            cursor_file: here.to_path_buf(),
            cursor_line1: self.line + 1,
        };
        let mut walked = Vec::new();
        let mut unknown = false;
        for ty in &around {
            let hits = self.cs_walk(ty, word, &level, 0, &mut walked, &mut unknown);
            if hits
                .iter()
                .any(|h| h.path == here && h.line1 == self.line + 1)
            {
                return Err(None);
            }
            let color = after.starts_with('.')
                && hits.iter().all(|h| {
                    matches!(search::cs_declared(&h.text, word),
                        search::CsValue::Type(t) if search::cs_type_name(&t).as_deref() == Some(word))
                });
            if color && !hits.is_empty() {
                return Err(None);
            }
            if !hits.is_empty() {
                return Ok(hits
                    .into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Path(ty.name.clone()),
                    })
                    .collect());
            }
        }
        Err((!unknown).then_some(walked))
    }

    fn cs_walk(
        &self,
        ty: &Typed,
        word: &str,
        level: &Level,
        depth: usize,
        walked: &mut Vec<String>,
        unknown: &mut bool,
    ) -> Vec<Hit> {
        walked.push(ty.name.clone());
        let Some(text) = self.text_of(&ty.path) else {
            *unknown = true;
            return Vec::new();
        };
        let (parts, _) = self.cs_parts(ty, &text);
        for part in &parts {
            let mut own = self.cs_members(part, word);
            let text = self.text_of(&part.path).unwrap_or_default();
            if own
                .iter()
                .any(|h| h.path == level.cursor_file && h.line1 == level.cursor_line1)
            {
                return own;
            }
            own.retain(|h| {
                self.cs_sees(&search::cs_place(&text, h.line1, word), &level.around)
                    && level.types.as_ref().is_none_or(|re| re.is_match(&h.text))
            });
            let own = self.cs_overload(word, self.cs_fit(word, level.args, None, own));
            if !own.is_empty() {
                return own;
            }
        }
        if depth >= 8 {
            *unknown = true;
            return Vec::new();
        }
        for part in &parts {
            let Some(text) = self.text_of(&part.path) else {
                continue;
            };
            for base in search::cs_bases(&text, part.line) {
                match self.cs_resolve(&part.path, &text, part.line, &base) {
                    Some(CsType::Project(b)) if !walked.contains(&b.name) => {
                        let found = self.cs_walk(&b, word, level, depth + 1, walked, unknown);
                        if !found.is_empty() {
                            return found;
                        }
                    }
                    Some(CsType::Project(_)) => {}
                    Some(CsType::Outside(name)) => walked.push(name),
                    None => *unknown = true,
                }
            }
        }
        Vec::new()
    }

    pub(super) fn cs_reachable(
        &self,
        here: &Path,
        word: &str,
        chain: Option<&[String]>,
        walked: Option<&[String]>,
        hits: Vec<Hit>,
    ) -> Vec<Hit> {
        let around = OnceCell::new();
        let walked = walked.and_then(|w| {
            let cut = self.truncated.get();
            let statics = self.cs_using_statics(here, w);
            self.truncated.set(cut);
            Some((w, statics?))
        });
        let mut hits = hits;
        hits.retain(|h| {
            let place = self.hit_text(h).map_or(search::CsPlace::Top, |t| {
                search::cs_place(&t, h.line1, word)
            });
            if !self.cs_sees(&place, &around) {
                return false;
            }
            match place {
                search::CsPlace::Member { owner, .. } => walked.as_ref().is_none_or(|(w, s)| {
                    w.contains(&owner)
                        || (s.contains(&owner) && search::cs_static_member(&h.text, word))
                }),
                search::CsPlace::Local { from, to } => {
                    chain.is_none()
                        && h.path == here
                        && (h.deleted.is_some() || (from..=to).contains(&(self.line + 1)))
                }
                search::CsPlace::Top => true,
            }
        });
        if !hits
            .iter()
            .any(|h| h.deleted.is_none() && h.path == here && h.line1 == self.line + 1)
        {
            let patterns = search::def_patterns(Kind::CSharp, word);
            let ty = Regex::new(&patterns[0]).expect("an escaped name keeps the pattern valid");
            let ctor = Regex::new(&patterns[4]).expect("an escaped name keeps the pattern valid");
            let typed: Vec<PathBuf> = (hits.iter())
                .filter(|h| ty.is_match(&h.text))
                .map(|h| h.path.clone())
                .collect();
            hits.retain(|h| {
                ty.is_match(&h.text) || !ctor.is_match(&h.text) || !typed.contains(&h.path)
            });
        }
        let hits = self.cs_type_namesakes(here, word, chain, hits);
        let receiver = chain.map(|c| c.last().map_or("", String::as_str));
        self.cs_fit(word, self.cs_args(), receiver, hits)
    }

    fn cs_using_statics(&self, here: &Path, walked: &[String]) -> Option<Vec<String>> {
        let msbuild = r#"<Using\b[^>]*\bStatic\s*=\s*"(?i:true)""#;
        let csproj = |p: &Path| p.extension().is_some_and(|e| e == "csproj");
        if !self.grep(msbuild, false, false, csproj).ok()?.is_empty() {
            return None;
        }
        let text = self.buf.lines.join("\n");
        let global = r"^\u{feff}?\s*global\s+using\s+static\b";
        let mut written: Vec<(PathBuf, String, usize, String)> = Vec::new();
        for (k, line) in self.buf.lines.iter().enumerate() {
            if line.contains("using static ") {
                let u = search::cs_using(line).filter(|u| u.fixed)?;
                written.push((here.to_path_buf(), text.clone(), k + 1, u.target));
            }
        }
        for h in self.project_grep(Kind::CSharp, here, global) {
            let u = search::cs_using(&h.text).filter(|u| u.fixed)?;
            written.push((h.path.clone(), self.text_of(&h.path)?, h.line1, u.target));
        }
        if written.is_empty() {
            return Some(Vec::new());
        }
        let around: Vec<String> = self.cs_around().into_iter().map(|(n, _)| n).collect();
        for name in walked.iter().filter(|n| !around.contains(n)) {
            let pattern = search::def_patterns(Kind::CSharp, name)[..4].join("|");
            let mut hits = self.project_definitions(Kind::CSharp, here, name, &pattern);
            hits.retain(|h| self.hidden_now(Kind::CSharp, h).get(h.line1 - 1) != Some(&true));
            if hits.len() != 1 {
                return None;
            }
        }
        let mut types = Vec::new();
        for (path, text, line1, target) in written {
            let Some(CsType::Project(t)) = self.cs_resolve(&path, &text, line1, &target) else {
                return None;
            };
            let decl = self
                .text_of(&t.path)?
                .lines()
                .nth(t.line - 1)
                .and_then(search::cs_type_decl)?;
            if decl.keyword == "enum" {
                return None;
            }
            types.push(t.name);
        }
        Some(types)
    }

    pub(super) fn cs_args(&self) -> Option<usize> {
        let (range, word) = self.definition_word(Some(Kind::CSharp))?;
        let declares = Regex::new(&search::def_patterns(Kind::CSharp, &word).join("|"))
            .is_ok_and(|re| re.is_match(self.line_str()));
        if declares && self.on_declared_name(Kind::CSharp, &word, true) {
            return None;
        }
        search::cs_arguments(&self.buf.lines, self.line, range.end)
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

    fn cs_overload(&self, word: &str, hits: Vec<Hit>) -> Vec<Hit> {
        let Some(args) = self
            .definition_word(Some(Kind::CSharp))
            .and_then(|(r, _)| search::cs_literal_arguments(&self.buf.lines, self.line, r.end))
        else {
            return hits;
        };
        search::literal_overloads(hits, &args, |h| {
            search::cs_parameter_keywords(&self.hit_text(h)?, h.line1, word)
        })
    }
}

struct Level {
    around: OnceCell<Vec<String>>,
    types: Option<Regex>,
    args: Option<usize>,
    cursor_file: PathBuf,
    cursor_line1: usize,
}

fn owner_of(file: &Path, text: &str, line1: usize) -> Option<Typed> {
    let decl = search::cs_owner(text, line1)?;
    let name = search::cs_type_decl(text.lines().nth(decl - 1)?)?.name;
    Some(Typed {
        name,
        path: file.to_path_buf(),
        line: decl,
    })
}

fn generic(written: &str) -> bool {
    written.contains('<') || written.trim_end_matches('?').ends_with("[]")
}

fn same(a: &CsType, b: &CsType) -> bool {
    match (a, b) {
        (CsType::Project(a), CsType::Project(b)) => a.path == b.path && a.line == b.line,
        (CsType::Outside(a), CsType::Outside(b)) => a == b,
        _ => false,
    }
}

fn one_more(found: &mut Option<Proven>, next: Proven) -> Option<()> {
    match found {
        Some(one) if !same(&one.ty, &next.ty) => return None,
        Some(one)
            if search::cs_one_argument(&one.written) != search::cs_one_argument(&next.written) =>
        {
            one.written = format!(
                "{}<?>",
                search::cs_type_name(&one.written).unwrap_or_default()
            );
        }
        Some(_) => {}
        None => *found = Some(next),
    }
    Some(())
}

fn receivers(hits: Vec<Hit>, label: &str) -> Vec<Candidate> {
    hits.into_iter()
        .map(|hit| Candidate {
            hit,
            reason: Reason::Receiver(label.to_owned()),
        })
        .collect()
}

fn partial_shape(name: &str) -> Regex {
    Regex::new(&format!(
        r"\bpartial\b.*?\b{}\s*(<[^>]*>)?\s*(\()?",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid")
}

fn part_shape(shape: &Regex, header: &str) -> Option<(String, bool)> {
    let c = shape.captures(header)?;
    let generics = c.get(1).map_or("", |m| m.as_str());
    Some((
        generics.replace(char::is_whitespace, ""),
        c.get(2).is_some(),
    ))
}

fn one_namespace(text: &str, line1: Option<usize>) -> bool {
    let mut namespaces = text.lines().filter(|l| search::cs_namespace_line(l));
    match (namespaces.next(), namespaces.next()) {
        (None, _) => true,
        (Some(only), None) => {
            only.trim_end().ends_with(';')
                || line1.is_none_or(|n| {
                    (text.lines().nth(n - 1)).is_some_and(|l| l.starts_with([' ', '\t']))
                })
        }
        _ => false,
    }
}
