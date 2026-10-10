use super::*;

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
}

struct CsWrittenType {
    ty: CsType,
    written: String,
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
        let Proven { mut ty, link } = self
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
            let Proven { ty: next, link } =
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

    fn cs_member_answer(
        &self,
        here: &Path,
        ty: &CsType,
        word: &str,
        label: &str,
    ) -> Option<CsAnswer> {
        let (walked, outside) = match ty {
            CsType::Outside(name) => (vec![name.clone()], true),
            CsType::Project(t) => match self.cs_up(t, word, 0) {
                Up::Found(hits) => {
                    let fit = self.cs_fit(word, self.cs_args(), Some(""), hits.clone());
                    let hits = if fit.is_empty() { hits } else { fit };
                    return Some(CsAnswer::Found(receivers(hits, label)));
                }
                Up::Outside { walked } => (walked, true),
                Up::Missing { walked } => (walked, false),
                Up::Unknown => return None,
            },
        };
        let w = regex::escape(word);
        let pattern = format!(r"\bstatic\b.*\b{w}\s*(?:<[^>]*>)?\s*\(\s*this\s");
        let extensions = self.project_grep(Kind::CSharp, here, &pattern);
        self.truncated.set(false);
        let (of, other): (Vec<Hit>, Vec<Hit>) = extensions.into_iter().partition(|h| {
            search::cs_extended(&h.text)
                .and_then(|t| search::cs_type_name(&t))
                .is_some_and(|t| walked.contains(&t))
        });
        match (of.is_empty(), other.is_empty()) {
            (false, _) => Some(CsAnswer::Found(receivers(of, label))),
            (true, true) if outside => Some(CsAnswer::Outside {
                links: label.to_owned(),
            }),
            _ => None,
        }
    }

    fn cs_up(&self, ty: &Typed, word: &str, depth: usize) -> Up {
        let own = self.cs_members(ty, word);
        if !own.is_empty() {
            return Up::Found(own);
        }
        let mut walked = vec![ty.name.clone()];
        let mut outside = false;
        // ponytail: eight levels up, which also ends a cycle.
        let Some(text) = self.text_of(&ty.path).filter(|_| depth < 8) else {
            return Up::Outside { walked };
        };
        for base in search::cs_bases(&text, ty.line) {
            match self.cs_resolve(&ty.path, &text, ty.line, &base) {
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
        let partial = Regex::new(r"\bpartial\b").is_ok_and(|re| {
            text.lines()
                .nth(ty.line - 1)
                .is_some_and(|l| re.is_match(l))
        });
        match (partial, outside) {
            (true, _) => Up::Unknown,
            (false, true) => Up::Outside { walked },
            (false, false) => Up::Missing { walked },
        }
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
        Some(Proven { ty, link })
    }

    fn cs_name_type(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        hops: usize,
    ) -> Option<Proven> {
        let owner = || {
            let decl = search::cs_owner(text, line1)?;
            let n = search::cs_type_decl(text.lines().nth(decl - 1)?)?.name;
            Some(Typed {
                name: n,
                path: file.to_path_buf(),
                line: decl,
            })
        };
        match name {
            "this" => {
                let t = owner()?;
                let link = format!("this: {}", t.name);
                return Some(Proven {
                    ty: CsType::Project(t),
                    link,
                });
            }
            "base" => {
                let t = owner()?;
                let base = search::cs_bases(text, t.line).into_iter().next()?;
                let ty = self.cs_resolve(file, text, t.line, &base)?;
                let link = format!("base: {}", search::cs_type_name(&base)?);
                return Some(Proven { ty, link });
            }
            _ => {}
        }
        let lines: Vec<&str> = text.lines().collect();
        let bindings = search::bindings(Kind::CSharp, text, line1, name);
        if bindings.is_empty() {
            return self.cs_field_type(&owner()?, name);
        }
        let mut found: Option<Proven> = None;
        for b in bindings {
            let at = search::cs_written_line(&lines, b.line1, name);
            let value = search::cs_declared(lines.get(at - 1)?, name);
            let CsWrittenType { ty, written } = match value {
                search::CsValue::Type(w) => CsWrittenType {
                    ty: self.cs_resolve(file, text, at, &w)?,
                    written: w,
                },
                search::CsValue::Call { callee, awaited } if hops > 0 => {
                    self.cs_call_type(file, text, at, &callee, awaited, hops - 1)?
                }
                _ => return None,
            };
            let link = format!("{name}: {}", search::cs_type_name(&written)?);
            match &found {
                Some(one) if !same(&one.ty, &ty) => return None,
                Some(_) => {}
                None => found = Some(Proven { ty, link }),
            }
        }
        found
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
            [] => {
                let decl = search::cs_owner(text, line1)?;
                let n = search::cs_type_decl(text.lines().nth(decl - 1)?)?.name;
                Typed {
                    name: n,
                    path: file.to_path_buf(),
                    line: decl,
                }
            }
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
        let written = match awaited {
            true => search::cs_awaited(&written)?.to_owned(),
            false => written,
        };
        let at = self.text_of(&hit.path)?;
        let ty = self.cs_resolve(&hit.path, &at, hit.line1, &written)?;
        Some(CsWrittenType { ty, written })
    }

    fn cs_resolve(&self, file: &Path, text: &str, line1: usize, written: &str) -> Option<CsType> {
        let name = search::cs_type_name(written)?;
        if search::cs_generic_param(text, line1, &name) {
            return None;
        }
        let cut = self.truncated.get();
        let pattern = search::def_patterns(Kind::CSharp, &name)[..4].join("|");
        let hits = self.project_definitions(Kind::CSharp, file, &name, &pattern);
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
            _ => None,
        };
        self.truncated.set(cut);
        found
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
        let (before, after) = (line[..r.start].trim_end(), line[r.end..].trim_start());
        if after.starts_with('=')
            && !after.starts_with("==")
            && !after.starts_with("=>")
            && (before.ends_with(['{', ','])
                || (before.is_empty() && !line.trim_end().ends_with(';')))
        {
            return Err(None);
        }
        let text = self.buf.lines.join("\n");
        let lines: Vec<&str> = text.lines().collect();
        let mut around = Vec::new();
        let mut at = search::cs_owner(&text, self.line + 1);
        while let Some(decl) = at.filter(|_| around.len() < 8) {
            let name = search::cs_type_decl(lines[decl - 1]).ok_or(None)?.name;
            around.push(Typed {
                name,
                path: here.to_path_buf(),
                line: decl,
            });
            at = search::cs_owner(&text, decl);
        }
        if around.is_empty() {
            return Err(None);
        }
        let level = Level {
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
        let mut parts = vec![ty.clone()];
        let Some(text) = self.text_of(&ty.path) else {
            *unknown = true;
            return Vec::new();
        };
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
            let namespace = search::cs_namespace(&text, ty.line);
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
        for part in &parts {
            let mut own = self.cs_members(part, word);
            let text = self.text_of(&part.path).unwrap_or_default();
            own.retain(|h| {
                !matches!(
                    search::cs_place(&text, h.line1, word),
                    search::CsPlace::Local { .. }
                )
            });
            if own
                .iter()
                .any(|h| h.path == level.cursor_file && h.line1 == level.cursor_line1)
            {
                return own;
            }
            own.retain(|h| {
                let private = matches!(
                    search::cs_place(&text, h.line1, word),
                    search::CsPlace::Member { private: true, .. }
                );
                !(depth > 0 && private)
                    && level.types.as_ref().is_none_or(|re| re.is_match(&h.text))
            });
            let own = self.cs_fit(word, level.args, None, own);
            if !own.is_empty() {
                return own;
            }
        }
        // ponytail: eight levels up, which also ends a cycle.
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
        let mine: Vec<String> = self
            .buf
            .lines
            .iter()
            .filter_map(|l| search::cs_type_decl(l).map(|d| d.name))
            .collect();
        let statics = r"^\u{feff}?\s*global\s+using\s+static\b";
        let walked = walked.filter(|_| {
            let cut = self.truncated.get();
            let none = self.project_grep(Kind::CSharp, here, statics).is_empty()
                && !self.buf.lines.iter().any(|l| l.contains("using static "));
            self.truncated.set(cut);
            none
        });
        let mut hits = hits;
        hits.retain(|h| {
            let place = self.text_of(&h.path).map_or(search::CsPlace::Top, |t| {
                search::cs_place(&t, h.line1, word)
            });
            match place {
                search::CsPlace::Member { owner, .. }
                    if walked.is_some_and(|w| !w.contains(&owner)) =>
                {
                    false
                }
                search::CsPlace::Member {
                    owner,
                    private: true,
                } => mine.contains(&owner),
                search::CsPlace::Local { from, to } => {
                    chain.is_none() && h.path == here && (from..=to).contains(&(self.line + 1))
                }
                _ => true,
            }
        });
        if !hits
            .iter()
            .any(|h| h.path == here && h.line1 == self.line + 1)
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
        let receiver = chain.map(|c| c.last().map_or("", String::as_str));
        self.cs_fit(word, self.cs_args(), receiver, hits)
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
                let Some(text) = self.text_of(&h.path) else {
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
}

struct Level {
    types: Option<Regex>,
    args: Option<usize>,
    cursor_file: PathBuf,
    cursor_line1: usize,
}

fn same(a: &CsType, b: &CsType) -> bool {
    match (a, b) {
        (CsType::Project(a), CsType::Project(b)) => a.path == b.path && a.line == b.line,
        (CsType::Outside(a), CsType::Outside(b)) => a == b,
        _ => false,
    }
}

fn receivers(hits: Vec<Hit>, label: &str) -> Vec<Candidate> {
    hits.into_iter()
        .map(|hit| Candidate {
            hit,
            reason: Reason::Receiver(label.to_owned()),
        })
        .collect()
}
