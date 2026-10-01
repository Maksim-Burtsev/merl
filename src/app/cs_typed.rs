//! C#'s typed lookup (#352): the type of a receiver as C# writes it, and of an object
//! initializer, and the members that type declares. And what a C# name can reach: the projects
//! its file compiles against (#349), the class around it first, a namespace on a `using` line,
//! an overload that takes the call's arguments (#360).

use super::*;

/// A C# type a name is proven to hold.
#[derive(Debug, Clone)]
enum CsType {
    /// Declared once in the project.
    Project(Typed),
    /// Declared nowhere in the project: the framework's or a package's, by this name.
    Outside(String),
}

/// What the C# rules answer for `x.word` or an initializer's `Word = …`.
pub(super) enum CsAnswer {
    /// The member in the project's type or the types above it, or an extension method of it.
    Found(Vec<Candidate>),
    /// The type, or every base the member could come from, is not the project's: the member is
    /// the framework's. The links that prove it.
    Outside(String),
}

/// What a member lookup up a type's hierarchy found.
enum Up {
    Found(Vec<Hit>),
    /// Not in the project's types; a base is outside the project and may declare it. The names of
    /// the types walked, for the extension methods.
    Outside(Vec<String>),
    /// Not in the project's types, which are all read: the rules missed it, or it is an
    /// extension method. The names of the types walked.
    Missing(Vec<String>),
    /// Not in the project's types, and a type the rules cannot read may declare it: a base
    /// they cannot tell, or a partial type's generated part.
    Unknown,
}

impl App {
    /// `word` on the C# receiver `chain`, `x.f.g`, each link proven (#352). `Err` names the name
    /// the chain broke at, and leaves the word to the search by name.
    pub(super) fn cs_typed(
        &self,
        here: &Path,
        word: &str,
        chain: &[String],
    ) -> Result<CsAnswer, String> {
        let text = self.buf.lines.join("\n");
        let (first, fields) = chain.split_first().ok_or_else(|| word.to_owned())?;
        let (mut ty, link) = self
            .cs_name_type(here, &text, self.line + 1, first, 1)
            .ok_or_else(|| first.clone())?;
        let mut links = vec![link];
        for (i, field) in fields.iter().enumerate() {
            // ponytail: six names in front of the word, as in the other languages.
            if i == 5 {
                return Err(field.clone());
            }
            let CsType::Project(t) = &ty else {
                return Err(field.clone());
            };
            let (next, link) = self.cs_field_type(t, field).ok_or_else(|| field.clone())?;
            links.push(link);
            ty = next;
        }
        let label = links.join(" \u{2192} ");
        self.cs_member_answer(here, &ty, word, &label)
            .ok_or_else(|| chain.last().cloned().unwrap_or_default())
    }

    /// `Word` of an object initializer on the cursor's line, `new T { Word = … }` (#352). `None`
    /// when the cursor is on no initializer's member, or its type is not proven.
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
            // An initializer sets a field or a property, never an extension method.
            CsType::Outside(_) => Some(CsAnswer::Outside(label)),
            CsType::Project(t) => match self.cs_up(&t, word, 0) {
                Up::Found(hits) => Some(CsAnswer::Found(receivers(hits, &label))),
                Up::Outside(_) => Some(CsAnswer::Outside(label)),
                Up::Missing(_) | Up::Unknown => None,
            },
        }
    }

    /// The member `word` of the type `ty`: in the project's type or above it, else an extension
    /// method of one of the types walked, else "outside" when a type it may come from is not the
    /// project's. `None` leaves it to the search by name.
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
                // An overload that cannot take the call's arguments is none (#360), unless none can.
                Up::Found(hits) => {
                    let fit = self.cs_fit(word, self.cs_args(), Some(""), hits.clone());
                    let hits = if fit.is_empty() { hits } else { fit };
                    return Some(CsAnswer::Found(receivers(hits, label)));
                }
                Up::Outside(walked) => (walked, true),
                Up::Missing(walked) => (walked, false),
                Up::Unknown => return None,
            },
        };
        // An extension method of one of those types. One the project declares for any other
        // type may still be it (`IEnumerable<T>` of a `List<T>`): that is left to the name.
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
            (true, true) if outside => Some(CsAnswer::Outside(label.to_owned())),
            _ => None,
        }
    }

    /// `word` declared in `ty`, else in the bases and interfaces its header names, nearest first.
    fn cs_up(&self, ty: &Typed, word: &str, depth: usize) -> Up {
        let own = self.cs_members(ty, word);
        if !own.is_empty() {
            return Up::Found(own);
        }
        let mut walked = vec![ty.name.clone()];
        let mut outside = false;
        // ponytail: eight levels up, which also ends a cycle.
        let Some(text) = self.text_of(&ty.path).filter(|_| depth < 8) else {
            return Up::Outside(walked);
        };
        for base in search::cs_bases(&text, ty.line) {
            match self.cs_resolve(&ty.path, &text, ty.line, &base) {
                Some(CsType::Project(b)) => match self.cs_up(&b, word, depth + 1) {
                    Up::Found(hits) => return Up::Found(hits),
                    Up::Outside(more) => {
                        outside = true;
                        walked.extend(more);
                    }
                    Up::Missing(more) => walked.extend(more),
                    Up::Unknown => return Up::Unknown,
                },
                Some(CsType::Outside(name)) => {
                    outside = true;
                    walked.push(name);
                }
                // A base the rules cannot tell may hold it: nothing is proven.
                None => return Up::Unknown,
            }
        }
        // A partial type may have a part a source generator writes (`[ObservableProperty]`),
        // which the project does not hold: what it lacks is not proven to be the framework's.
        let partial = Regex::new(r"\bpartial\b").is_ok_and(|re| {
            text.lines()
                .nth(ty.line - 1)
                .is_some_and(|l| re.is_match(l))
        });
        match (partial, outside) {
            (true, _) => Up::Unknown,
            (false, true) => Up::Outside(walked),
            (false, false) => Up::Missing(walked),
        }
    }

    /// The declarations of `word` directly in the body of the C# type `ty`: a method, a property,
    /// a field, an event, a nested type, or a positional record's parameter.
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
            line,
            col: 0,
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

    /// The type of the field or property `field` that `ty`, or a type above it, declares.
    fn cs_field_type(&self, ty: &Typed, field: &str) -> Option<(CsType, String)> {
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
        let found = self.cs_resolve(&hit.path, &text, hit.line, &written)?;
        let link = format!("{field}: {}", search::cs_type_name(&written)?);
        Some((found, link))
    }

    /// The type of the name `name` on 1-based `line` of `text`, the text of `file`, and the link
    /// that proves it: `this`, `base`, a local or a parameter the scopes around bind, else a field
    /// or a property of the type around the line.
    fn cs_name_type(
        &self,
        file: &Path,
        text: &str,
        line: usize,
        name: &str,
        hops: usize,
    ) -> Option<(CsType, String)> {
        let owner = || {
            let decl = search::cs_owner(text, line)?;
            let (_, n) = search::cs_type_decl(text.lines().nth(decl - 1)?)?;
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
                return Some((CsType::Project(t), link));
            }
            "base" => {
                let t = owner()?;
                let base = search::cs_bases(text, t.line).into_iter().next()?;
                let found = self.cs_resolve(file, text, t.line, &base)?;
                return Some((found, format!("base: {}", search::cs_type_name(&base)?)));
            }
            _ => {}
        }
        let lines: Vec<&str> = text.lines().collect();
        let bindings = search::bindings(Kind::CSharp, text, line, name);
        if bindings.is_empty() {
            return self.cs_field_type(&owner()?, name);
        }
        let mut found: Option<(CsType, String)> = None;
        for b in bindings {
            let at = search::cs_written_line(&lines, b.line, name);
            let value = search::cs_declared(lines.get(at - 1)?, name);
            let (ty, written) = match value {
                search::CsValue::Type(w) => (self.cs_resolve(file, text, at, &w)?, w),
                search::CsValue::Call { callee, awaited } if hops > 0 => {
                    self.cs_call_type(file, text, at, &callee, awaited, hops - 1)?
                }
                _ => return None,
            };
            let link = format!("{name}: {}", search::cs_type_name(&written)?);
            match &found {
                Some((one, _)) if !same(one, &ty) => return None,
                Some(_) => {}
                None => found = Some((ty, link)),
            }
        }
        found
    }

    /// What a call of `callee` on 1-based `line` of `file` gives: the declared return type of the
    /// one method of the name that the type around the line, the receiver's type or the type the
    /// callee names declares; `Task<T>` and `ValueTask<T>` under `await` read as `T`. The type,
    /// and the return type as written.
    fn cs_call_type(
        &self,
        file: &Path,
        text: &str,
        line: usize,
        callee: &str,
        awaited: bool,
        hops: usize,
    ) -> Option<(CsType, String)> {
        let parts: Vec<&str> = callee.split('.').collect();
        let (method, receiver) = parts.split_last()?;
        let owner = match receiver {
            [] => {
                let decl = search::cs_owner(text, line)?;
                let (_, n) = search::cs_type_decl(text.lines().nth(decl - 1)?)?;
                Typed {
                    name: n,
                    path: file.to_path_buf(),
                    line: decl,
                }
            }
            [one] => match self.cs_name_type(file, text, line, one, hops) {
                Some((CsType::Project(t), _)) => t,
                Some((CsType::Outside(_), _)) => return None,
                // `Factory.Create()`: a static method of a type the project declares.
                None => match self.cs_resolve(file, text, line, one)? {
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
        let ty = self.cs_resolve(&hit.path, &at, hit.line, &written)?;
        Some((ty, written))
    }

    /// The type written as `written` on 1-based `line` of `text`, the text of `file`: the one
    /// type of that name the project declares, or one it declares nowhere. `None` for a type
    /// parameter, `dynamic`, an array, and a name the project declares more than once or as
    /// something else (an alias, a namespace).
    fn cs_resolve(&self, file: &Path, text: &str, line: usize, written: &str) -> Option<CsType> {
        let name = search::cs_type_name(written)?;
        if search::cs_generic_param(text, line, &name) {
            return None;
        }
        let cut = self.truncated.get();
        // What declares a type-like name: a type, a delegate, a namespace, a `using` alias; a
        // property called `Label` is no declaration of the type `Label`.
        let pattern = search::def_patterns(Kind::CSharp, &name)[..4].join("|");
        let hits = self.project_definitions(Kind::CSharp, file, &name, &pattern);
        let (types, other): (Vec<&Hit>, Vec<&Hit>) = hits
            .iter()
            .partition(|h| search::cs_type_decl(&h.text).is_some_and(|(_, n)| n == name));
        let found = match (types.as_slice(), other.is_empty()) {
            ([one], true) => Some(CsType::Project(Typed {
                name: name.clone(),
                path: one.path.clone(),
                line: one.line,
            })),
            ([], true) => Some(CsType::Outside(name)),
            _ => None,
        };
        self.truncated.set(cut);
        found
    }
}

impl App {
    /// The projects a C# file `here` sees (#349); `None` for another kind, or when they cannot be
    /// told and every file stays in sight.
    pub(super) fn cs_sight(&self, kind: Kind, here: &Path) -> Option<search::CsProjects> {
        if kind != Kind::CSharp {
            return None;
        }
        search::cs_projects(&self.files, here, |p| {
            std::fs::read_to_string(self.root.join(p)).ok()
        })
    }

    /// Whether the C# project declares `name` in any form (#355): a rule of `d` matches it, or a
    /// `namespace` line has it as one of its parts.
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

    /// A C# `Offer.Cut` whose qualifier is an `enum` of the project: the member its body lists
    /// (#466). A bare `Cut` has no rule: in C# only a `using static` brings it in. The enum's
    /// namespace has to be one this file sees, or a namesake the SDK brings in (MAUI's
    /// `PermissionStatus`) would pass for the project's. `path` is the chain as written.
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
                let (line, col) = search::enum_member(kind, &text, h.line, word)?;
                let hit = Hit {
                    deleted: None,
                    text: text.lines().nth(line - 1)?.to_owned(),
                    path: h.path,
                    line,
                    col,
                };
                let reason = Reason::Path(path.to_owned());
                Some((Candidate { hit, reason }, search::cs_namespace(&text, line)))
            })
            .collect();
        let prefix = chain[..chain.len() - 1].join(".");
        let inside = search::cs_namespace(text, self.line + 1);
        let mut opened = search::cs_usings(text);
        if !members.is_empty() {
            // `global using` in any file of the `.csproj` this file is in, `<Using Include>` in
            // that `.csproj`: a solution's other projects open their own.
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

    /// On a namespace segment (#360), the `namespace` lines in sight that declare that namespace
    /// or one inside it, and never a type or a member. `None` off a segment, and in a `global::`
    /// path that no namespace answers: its last name may be a type.
    pub(super) fn cs_namespace_segment(
        &self,
        here: &Path,
        range: &std::ops::Range<usize>,
    ) -> Option<Vec<Candidate>> {
        let (prefix, certain) =
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

    /// Whether the bare C# `word` at `range` of the cursor's line is looked up as a type alone
    /// (#360). After `is` a constant pattern may stand too, `x is Max` with a `const int Max`
    /// (#581): a type of the name in sight still wins, and with none it is the search by name.
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
        let is = line[..range.start].trim_end().strip_suffix("is");
        if is.is_none_or(|b| b.ends_with(|c: char| c.is_alphanumeric() || c == '_')) {
            return true;
        }
        let patterns = search::def_patterns(Kind::CSharp, word);
        let types = [&patterns[0], &patterns[1], &patterns[3]].map(String::as_str);
        let cut = self.truncated.get();
        let hits = self.project_definitions(Kind::CSharp, here, word, &types.join("|"));
        let found = !self.cs_reachable(here, word, None, None, hits).is_empty();
        self.truncated.set(cut);
        found
    }

    /// A bare C# `word` inside a type (#360), looked up as C# resolves a simple name: in the type
    /// around the cursor, a `partial` part of it, the bases its header names, walked up, then the
    /// same for each type around that one. Only types count where a type stands. `Err` when none
    /// declares it, with the names of the types walked, or `None` when a base the rules cannot
    /// tell may declare it, or the cursor is in no type.
    pub(super) fn cs_class_first(
        &self,
        here: &Path,
        word: &str,
        types_only: bool,
    ) -> Result<Vec<Candidate>, Option<Vec<String>>> {
        // A member an initializer sets, `new { Checked = c }`, is no name read in the type.
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
            let (_, name) = search::cs_type_decl(lines[decl - 1]).ok_or(None)?;
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
            on: (here.to_path_buf(), self.line + 1),
        };
        let mut walked = Vec::new();
        let mut unknown = false;
        for ty in &around {
            let hits = self.cs_walk(ty, word, &level, 0, &mut walked, &mut unknown);
            // On a declaration of the name, its namesakes are offered, as before.
            if hits
                .iter()
                .any(|h| h.path == here && h.line == self.line + 1)
            {
                return Err(None);
            }
            // "Color Color": `Status.Open` in a type whose property `Status` is of the type
            // `Status` may mean either; today's lookup offers both.
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

    /// `word` declared in `ty`, a `partial` part of it, or up the bases they name, nearest first;
    /// every type passed is added to `walked`, and `unknown` set at a base the rules cannot tell.
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
            // A part is declared in the type's own namespace: `Shop.B.Page` is another type.
            let namespace = search::cs_namespace(&text, ty.line);
            for h in self.project_grep(Kind::CSharp, &ty.path, &pattern) {
                let same = self
                    .text_of(&h.path)
                    .is_some_and(|t| search::cs_namespace(&t, h.line) == namespace);
                if same && (h.path != ty.path || h.line != ty.line) {
                    parts.push(Typed {
                        name: ty.name.clone(),
                        path: h.path,
                        line: h.line,
                    });
                }
            }
            self.truncated.set(cut);
        }
        for part in &parts {
            // A local of one of its methods is no member: another method does not see it.
            let mut own = self.cs_members(part, word);
            let text = self.text_of(&part.path).unwrap_or_default();
            own.retain(|h| {
                !matches!(
                    search::cs_place(&text, h.line, word),
                    search::CsPlace::Local { .. }
                )
            });
            if own.iter().any(|h| (h.path.clone(), h.line) == level.on) {
                return own;
            }
            // A base's private member is out of reach, and so is an overload that cannot take
            // the call's arguments: the walk goes on up to the bases then (#360).
            own.retain(|h| {
                let private = matches!(
                    search::cs_place(&text, h.line, word),
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

    /// Of `hits` for a C# `word`, those the cursor can reach. A private member from its own type
    /// alone, a part of it this file declares included, and a local from its own method alone,
    /// never behind a dot, the `chain` in front of a dotted word (#355). A method whose
    /// parameters cannot take the call's arguments is no candidate (#360), nor, for a bare word
    /// whose types around were all `walked`, a member of any other type: C# reaches it through a
    /// qualifier or a `using static` alone.
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
            .filter_map(|l| search::cs_type_decl(l).map(|(_, name)| name))
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
            let place = self
                .text_of(&h.path)
                .map_or(search::CsPlace::Top, |t| search::cs_place(&t, h.line, word));
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
        // A constructor yields to its type declared in its file (#360): a use names the type. On
        // a declaration of the name, both stay its namesakes.
        if !hits
            .iter()
            .any(|h| h.path == here && h.line == self.line + 1)
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

    /// How many arguments the call of the C# word under the cursor passes (#360); `None` off a
    /// call, or for a list not read.
    pub(super) fn cs_args(&self) -> Option<usize> {
        let (range, word) = self.definition_word(Some(Kind::CSharp))?;
        // The name a declaration declares is followed by its parameters, not by arguments.
        let declares = Regex::new(&search::def_patterns(Kind::CSharp, &word).join("|"))
            .is_ok_and(|re| re.is_match(self.line_str()));
        if declares && self.on_declared_name(Kind::CSharp, &word, true) {
            return None;
        }
        search::cs_arguments(&self.buf.lines, self.line, range.end)
    }

    /// Of `hits` for a C# `word`, all but the methods whose parameters cannot take `args`
    /// arguments (#360). An extension's `this` is passed by the `receiver` of a call `x.word(…)`,
    /// `""` for a value, unless the receiver names the extension's own static class. A method
    /// whose list is not read stays, and so does everything with no call to count.
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
                let Some((min, max, this)) = search::cs_parameters(&text, h.line, word) else {
                    return true;
                };
                let owner = match search::cs_place(&text, h.line, word) {
                    search::CsPlace::Member { owner, .. } => owner,
                    _ => String::new(),
                };
                let by = usize::from(this && receiver.is_some_and(|q| q.is_empty() || q != owner));
                n + by >= min && max.is_none_or(|m| n + by <= m)
            })
            .collect()
    }
}

/// What a level of the C# class-first walk keeps (#360): only types where a type stands, only
/// overloads that take the call's `args`, and the cursor's own line, a declaration, as is.
struct Level {
    types: Option<Regex>,
    args: Option<usize>,
    on: (PathBuf, usize),
}

/// Whether two proven types are one.
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
