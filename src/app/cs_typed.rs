//! C#'s typed lookup (#352): the type of a receiver as C# writes it, and of an object
//! initializer, and the members that type declares.

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
                Up::Found(hits) => return Some(CsAnswer::Found(receivers(hits, label))),
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
