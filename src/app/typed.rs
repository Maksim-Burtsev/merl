//! The type of a receiver: what a value was declared, assigned or returned as.

use super::*;

struct TypeVia {
    ty: Typed,
    call_signature: Option<String>,
}

impl App {
    pub(super) fn typed_definitions(
        &mut self,
        kind: Kind,
        here: &Path,
        word: &str,
        chain: &[String],
        head: Option<&search::CallHead>,
    ) -> Result<Vec<Candidate>, String> {
        if kind == Kind::Python {
            self.external_files(kind);
        }
        let (text, line) = self.scope(here, chain.first())?;
        // `super().m()` / `super.m()` is `self` / `this` with the walk started one level up.
        if chain.first().is_some_and(|f| f == "super") {
            let [_] = chain else {
                return Err(chain[1].clone());
            };
            let TypeVia { ty, .. } = self
                .value_type(kind, here, &text, line, &chain[0], 1)
                .ok_or_else(|| chain[0].clone())?;
            let hits = self.above(kind, &ty, word, 0)?;
            let label = format!("super of {}", ty.name);
            return Ok(hits
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect());
        }
        if kind == Kind::TsJs
            && head.is_none()
            && let Some(found) = self.literal_member(kind, here, chain, word)
        {
            return Ok(found);
        }
        let (ty, links) = self.receiver(kind, here, chain, head)?;
        let declared = self.hierarchy(kind, &ty, 0, &mut |t| {
            let members = self.members_of(kind, t, word);
            if members.is_empty() {
                self.field_of(kind, t, word, false).map(|hit| vec![hit])
            } else {
                Some(members)
            }
        });
        // An assignment inside a method stands for a field no type declares, and the base-most
        // one introduced it: `self.audit = None` in a subclass is a use of the base's field.
        let hits = declared
            .or_else(|| {
                let mut assigned = None;
                self.hierarchy(kind, &ty, 0, &mut |t| {
                    if let Some(hit) = self.field_of(kind, t, word, true) {
                        assigned = Some(vec![hit]);
                    }
                    None::<()>
                });
                assigned
            })
            .unwrap_or_default();
        let label = links.join(" \u{2192} ");
        Ok(hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Receiver(label.clone()),
            })
            .collect())
    }

    fn scope(&self, here: &Path, first: Option<&String>) -> Result<(String, usize), String> {
        if let Some(scope) = self.script_scope(here, first.map(String::as_str)) {
            return Ok(scope);
        }
        match self.on_template(here) {
            true => Err(first.cloned().unwrap_or_default()),
            false => Ok((self.buf.lines.join("\n"), self.line + 1)),
        }
    }

    fn receiver(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        head: Option<&search::CallHead>,
    ) -> Result<(Typed, Vec<String>), String> {
        let (text, line) = self.scope(here, chain.first())?;
        match head {
            // The chain hangs off a call: `make_uow().users.word`.
            Some(search::CallHead {
                call_without_arguments: call,
                value,
                fields_to_word: fields,
            }) => {
                let value = value.clone();
                // A cast is its own link, as written: `via (repo as UserRepository)`.
                let cast = matches!(value, search::Value::Type(_) | search::Value::Cast(..))
                    .then(|| call.clone());
                let TypeVia { ty, call_signature } = self
                    .binding_type(
                        kind,
                        here,
                        &text,
                        &search::Binding { line1: line, value },
                        1,
                    )
                    .ok_or_else(|| call.clone())?;
                let start = TypeVia {
                    ty,
                    call_signature: call_signature.or(cast),
                };
                self.follow(kind, start, call, false, fields)
            }
            None => self.chain_type(kind, here, &text, line, chain, 1),
        }
    }

    pub(super) fn builtin_receiver(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        head: Option<&search::CallHead>,
    ) -> Option<String> {
        if kind != Kind::Python || head.is_some() {
            return None;
        }
        let (last, before) = chain.split_last()?;
        let (file, text, bindings, mut links) = self.last_bindings(kind, here, chain)?;
        let imports = search::imports(kind, &text);
        let builtin = |written: &str| {
            let parts = search::type_path(kind, written)?;
            let [name] = parts.as_slice() else {
                return None;
            };
            (search::PYTHON_BUILTIN_TYPES.contains(&name.as_str())
                && bound(&imports, name).is_none()
                && self.declaration(kind, &file, &parts).is_none())
            .then(|| name.clone())
        };
        let mut found: Option<(String, Option<String>)> = None;
        for b in &bindings {
            let this = match &b.value {
                search::Value::Type(t) => (builtin(t)?, None),
                search::Value::Call(callee) => {
                    let first = callee.split('.').next().unwrap_or(callee);
                    if hidden(kind, &file, &text, b.line1, first) {
                        return None;
                    }
                    let (t, at) = self.declared_return(kind, &file, callee)?;
                    let written = self.text_of(&at).map(|t2| search::imports(kind, &t2));
                    let name = builtin(&t)?;
                    if written.is_some_and(|i| bound(&i, &name).is_some()) {
                        return None;
                    }
                    let link = signature(kind, callee, &name);
                    (name, Some(link))
                }
                _ => return None,
            };
            match &found {
                Some((name, _)) if *name != this.0 => return None,
                Some(_) => {}
                None => found = Some(this),
            }
        }
        let (name, call) = found?;
        let link = call.unwrap_or_else(|| match links.first() {
            Some(_) if before.len() == 1 => format!("{}.{last}: {name}", before[0]),
            _ => format!("{last}: {name}"),
        });
        match (before.len(), links.first_mut()) {
            (1, Some(first)) => *first = link,
            _ => links.push(link),
        }
        Some(links.join(" \u{2192} "))
    }

    fn last_bindings(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
    ) -> Option<(PathBuf, String, Vec<search::Binding>, Vec<String>)> {
        let (last, before) = chain.split_last()?;
        match before {
            [] => {
                let text = self.buf.lines.join("\n");
                let bindings = search::bindings(kind, &text, self.line + 1, last);
                Some((here.to_path_buf(), text, bindings, Vec::new()))
            }
            _ => {
                let (ty, links) = self.receiver(kind, here, before, None).ok()?;
                let found = self.hierarchy(kind, &ty, 0, &mut |t| {
                    let text = self.text_of(&t.path)?;
                    let bindings = search::field_bindings(kind, &text, t.line, last);
                    (!bindings.is_empty()).then(|| (t.path.clone(), text, bindings))
                })?;
                Some((found.0, found.1, found.2, links))
            }
        }
    }

    fn literal_member(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let (file, text, bindings, mut links) = self.last_bindings(kind, here, chain)?;
        let [b] = &bindings[..] else {
            return None;
        };
        let search::Value::Type(written) = &b.value else {
            return None;
        };
        let (line, literal) = search::ts_literal_member(&text, b.line1, written, word)?;
        links.push(format!("{}: {literal}", chain.last()?));
        let hit = Hit {
            text: text.lines().nth(line - 1)?.to_owned(),
            path: file,
            line,
            col: 0,
            deleted: None,
        };
        Some(vec![Candidate {
            hit,
            reason: Reason::Receiver(links.join(" \u{2192} ")),
        }])
    }

    /// `None` when a base cannot be read at all: a call (`six.with_metaclass(…)`), a name nothing
    /// binds, a `*` import.
    pub(super) fn inherited_outside(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        head: Option<&search::CallHead>,
    ) -> Option<Typed> {
        if kind != Kind::Python || chain.first().is_some_and(|f| f == "super") {
            return None;
        }
        let (ty, _) = self.receiver(kind, here, chain, head).ok()?;
        self.ancestry_outside(&ty, 0)?.then_some(ty)
    }

    /// Whether a base of `ty`, or of a project class above it, is imported from outside the
    /// project; `None` when a base cannot be read.
    fn ancestry_outside(&self, ty: &Typed, depth: usize) -> Option<bool> {
        let kind = Kind::Python;
        // ponytail: eight levels up, which also ends a cycle.
        let text = self.text_of(&ty.path).filter(|_| depth < 8)?;
        let imports = search::imports(kind, &text);
        let mut outside = false;
        for base in search::bases(kind, &text, ty.line) {
            let parts = search::type_path(kind, &base)?;
            let (name, chain) = parts.split_last()?;
            // What `above` reads as declaring nothing worth a jump.
            if matches!(name.as_str(), "object" | "Generic" | "Protocol" | "ABC") {
                continue;
            }
            if let Some(base) = self.type_decl(kind, &ty.path, &base) {
                outside |= base.path.is_absolute() || self.ancestry_outside(&base, depth + 1)?;
                continue;
            }
            let path = bound(&imports, &parts[0])?;
            if path[0].starts_with('.')
                || self
                    .imported_definitions(kind, &ty.path, name, chain, &path)
                    .is_some()
            {
                return None;
            }
            outside = true;
        }
        Some(outside)
    }

    /// `word` as the class `chain` names declares it for the class itself: a method, else a line
    /// of the class body (`CONST = 1`, `RED = 1` of an `Enum`, a dataclass's `x: int`, a
    /// property), in the class or what [`App::above`] proves over it. An attribute a method
    /// assigns to `self` is an instance's, and no answer here.
    pub(super) fn class_attribute(
        &mut self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        word: &str,
    ) -> Vec<Candidate> {
        if kind == Kind::Python {
            self.external_files(kind);
        }
        let Some(ty) = self.type_decl(kind, here, &chain.join(".")) else {
            return Vec::new();
        };
        // Above the class the bases are read as `super` reads them: several that disagree, or
        // one outside the project, prove nothing.
        // Reading no declaration in the class is no proof that it has none: a tuple target, a
        // `def` under an `if`, a nested class. A body that writes the word other than behind a
        // `.` leaves it to the rules below and the search by name.
        let members = self.members_of(kind, &ty, word);
        let hits = match (members.is_empty(), self.field_of(kind, &ty, word, false)) {
            (false, _) => members,
            (true, Some(hit)) => vec![hit],
            (true, None) if self.class_writes(&ty, word) => Vec::new(),
            (true, None) => self.above(kind, &ty, word, 0).unwrap_or_default(),
        };
        hits.into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::Path(chain.join(".")),
            })
            .collect()
    }

    /// Whether the Python class `ty` writes `word` other than behind a `.`: on its header's
    /// lines, which a one-line body shares, or in its body, which a comment, a string's text or
    /// a closer at the margin does not end.
    fn class_writes(&self, ty: &Typed, word: &str) -> bool {
        let Some(text) = self.text_of(&ty.path) else {
            return true;
        };
        let bare = Regex::new(&format!(r"(?:^|[^\w.]){}\b", regex::escape(word)))
            .expect("an escaped name keeps the pattern valid");
        let literal = search::literal_lines(Kind::Python, &text);
        let indent = |l: &str| l.len() - l.trim_start().len();
        let lines: Vec<&str> = text.lines().collect();
        let base = lines.get(ty.line - 1).map_or(0, |l| indent(l));
        let mut header = true;
        for (i, l) in lines.iter().enumerate().skip(ty.line - 1) {
            let t = l.trim();
            let inert =
                t.is_empty() || t.starts_with(['#', ')', ']']) || literal.get(i) == Some(&true);
            if !header && !inert && indent(l) <= base {
                break;
            }
            if bare.is_match(l) && !(i == ty.line - 1 && t.starts_with(&format!("class {word}"))) {
                return true;
            }
            // The header ends on the line whose code ends in `:`, or holds the body behind it.
            header = header && !l.split('#').next().unwrap_or(l).contains(':');
        }
        false
    }

    /// The type of the chain `x.f.g` on `line1` of `text`, the text of `file`, and the links
    /// that prove it; `Err` names the first name that is not proven.
    fn chain_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        chain: &[String],
        hops: usize,
    ) -> Result<(Typed, Vec<String>), String> {
        let start = self
            .value_type(kind, file, text, line1, &chain[0], hops)
            .ok_or_else(|| chain[0].clone())?;
        self.follow(kind, start, &chain[0], true, &chain[1..])
    }

    /// From the type `start` of `name`, each of `fields` in the type before it. What the status
    /// line lists: `repo: UserRepository`, or with `merge` `self.uow: UnitOfWork` for the receiver
    /// and its field, then `users: UserRepository` for each field after them.
    fn follow(
        &self,
        kind: Kind,
        start: TypeVia,
        name: &str,
        merge: bool,
        fields: &[String],
    ) -> Result<(Typed, Vec<String>), String> {
        let TypeVia {
            mut ty,
            call_signature,
        } = start;
        let mut links = vec![call_signature.unwrap_or_else(|| format!("{name}: {}", ty.name))];
        for (i, field) in fields.iter().enumerate() {
            // ponytail: six names in front of the word; a longer chain breaks at the seventh.
            if i == 5 {
                return Err(field.clone());
            }
            let TypeVia {
                ty: next,
                call_signature,
            } = self
                .hierarchy(kind, &ty, 0, &mut |t| self.field_type(kind, t, field))
                .flatten()
                .ok_or_else(|| field.clone())?;
            let first = i == 0 && merge;
            let label = match first {
                true => format!("{name}.{field}"),
                false => field.clone(),
            };
            let link = call_signature.unwrap_or_else(|| format!("{label}: {}", next.name));
            if first {
                links[0] = link;
            } else {
                links.push(link);
            }
            ty = next;
        }
        Ok((ty, links))
    }

    /// The type of `name` on `line1` of `text`, the text of `file`: the one type all its
    /// declarations in scope read. `hops` is how many times a declaration may hand over to another
    /// name (`self.repo = repo`).
    fn value_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        hops: usize,
    ) -> Option<TypeVia> {
        let bindings = self.bindings_of(kind, file, text, line1, name);
        if !bindings.is_empty() || kind != Kind::Go {
            return self.agree(kind, file, text, &bindings, hops);
        }
        // A Go name no scope of the file declares is the package's, declared in any of its
        // files (#100); each is read in the file that writes it, and they have to agree. An
        // empty list is no proof of that: the walk misses locals, so the function around the
        // line must not so much as mention the name, and an import of the file is no variable.
        let imported = search::imports(kind, text).iter().any(|i| i.name == name);
        if imported || search::go_may_declare(text, line1, name) {
            return None;
        }
        let mut found: Option<TypeVia> = None;
        for f in self.package_files(kind, file) {
            let Some(text) = self.text_of(&f) else {
                continue;
            };
            let bindings = search::package_bindings(&text, name);
            if bindings.is_empty() {
                continue;
            }
            let this = self.agree(kind, &f, &text, &bindings, hops)?;
            match &found {
                Some(one) if (&one.ty.path, one.ty.line) != (&this.ty.path, this.ty.line) => {
                    return None;
                }
                Some(_) => {}
                None => found = Some(this),
            }
        }
        found
    }

    /// The field `field` as `ty` itself declares it: `None` when it does not, `Some(None)` when
    /// its declarations cannot be read or disagree.
    fn field_type(&self, kind: Kind, ty: &Typed, field: &str) -> Option<Option<TypeVia>> {
        let text = self.text_of(&ty.path)?;
        let bindings = search::field_bindings(kind, &text, ty.line, field);
        (!bindings.is_empty()).then(|| self.agree(kind, &ty.path, &text, &bindings, 1))
    }

    /// The one type every binding reads, or `None` when there is none, one cannot be read or
    /// two disagree. A binding already being read further out is left to that reading: inside
    /// it the others say what the name holds, and `self.model = self.model.to(device)` reads
    /// `to` on the type `self.model = model` gives.
    fn agree(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        bindings: &[search::Binding],
        hops: usize,
    ) -> Option<TypeVia> {
        let mut found: Option<TypeVia> = None;
        for b in bindings {
            let at = (file.to_path_buf(), b.line1);
            if self.reading.borrow().contains(&at) {
                continue;
            }
            self.reading.borrow_mut().push(at);
            let this = self.binding_type(kind, file, text, b, hops);
            self.reading.borrow_mut().pop();
            let this = this?;
            match &found {
                Some(one) if (&one.ty.path, one.ty.line) != (&this.ty.path, this.ty.line) => {
                    return None;
                }
                Some(_) => {}
                None => found = Some(this),
            }
        }
        found
    }

    fn binding_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        b: &search::Binding,
        hops: usize,
    ) -> Option<TypeVia> {
        match &b.value {
            search::Value::Type(t) | search::Value::New(t) => {
                // `new api.Tool()` on a parameter `api` names no namespace of the file.
                let path = search::type_path(kind, t).filter(|p| p.len() > 1);
                if path.is_some_and(|p| hidden(kind, file, text, b.line1, &p[0])) {
                    return None;
                }
                self.type_decl(kind, file, t).map(|ty| TypeVia {
                    ty,
                    call_signature: None,
                })
            }
            search::Value::Class { decl_line1: line } => {
                let decl = text.lines().nth(line - 1)?;
                let name = decl
                    .split("class")
                    .nth(1)?
                    .trim_start()
                    .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                    .next()
                    .filter(|n| !n.is_empty())?;
                let ty = Typed {
                    name: name.to_owned(),
                    path: file.to_path_buf(),
                    line: *line,
                };
                Some(TypeVia {
                    ty,
                    call_signature: None,
                })
            }
            search::Value::Call(callee) => self.call_type(kind, file, text, b.line1, callee, hops),
            search::Value::Struct(line) => Some(TypeVia {
                ty: anonymous(file, *line),
                call_signature: None,
            }),
            // `cast(T, x)` writes `T`, unless the project declares the `cast` this file calls:
            // that one is a function, with whatever it returns.
            search::Value::Cast(callee, t) => {
                let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
                match self.declaration(kind, file, &parts) {
                    Some(_) => self.call_type(kind, file, text, b.line1, callee, hops),
                    None => self.type_decl(kind, file, t).map(|ty| TypeVia {
                        ty,
                        call_signature: None,
                    }),
                }
            }
            search::Value::Name(n) if hops > 0 => {
                self.value_type(kind, file, text, b.line1, n, hops - 1)
            }
            // An element of a collection whose every declaration writes its type: an
            // annotation, or the return type of the function it was assigned from.
            search::Value::Element(n) if hops > 0 => {
                let (file, text, bindings) = match n.rsplit_once('.') {
                    None => (
                        file.to_path_buf(),
                        text.to_owned(),
                        self.bindings_of(kind, file, text, b.line1, n),
                    ),
                    Some((chain, last)) => {
                        let chain: Vec<String> = chain.split('.').map(str::to_owned).collect();
                        let (ty, _) = self
                            .chain_type(kind, file, text, b.line1, &chain, hops - 1)
                            .ok()?;
                        self.hierarchy(kind, &ty, 0, &mut |t| {
                            let text = self.text_of(&t.path)?;
                            let bindings = search::field_bindings(kind, &text, t.line, last);
                            (!bindings.is_empty()).then(|| (t.path.clone(), text, bindings))
                        })?
                    }
                };
                let (file, text) = (file.as_path(), text.as_str());
                let mut found: Option<TypeVia> = None;
                for c in bindings {
                    let (written, at, link) = match &c.value {
                        search::Value::Type(t) => {
                            (t.clone(), file.to_path_buf(), format!("{n}: {}", t.trim()))
                        }
                        search::Value::Call(callee) => {
                            let first = callee.split('.').next().unwrap_or(callee);
                            if hidden(kind, file, text, c.line1, first) {
                                return None;
                            }
                            let (t, at) = self.declared_return(kind, file, callee)?;
                            let link = signature(kind, callee, &t);
                            (t, at, link)
                        }
                        _ => return None,
                    };
                    // Go's `type IssueList []*Issue` says what it holds on its own line.
                    let named = |written: &str| {
                        let list = self
                            .type_decl(kind, &at, written)
                            .filter(|_| kind == Kind::Go)?;
                        let text = self.text_of(&list.path)?;
                        let decl = text
                            .lines()
                            .nth(list.line - 1)?
                            .trim()
                            .strip_prefix("type ")?;
                        let holds = decl.trim_start().strip_prefix(list.name.as_str())?;
                        Some((search::element_type(kind, holds)?, list.path))
                    };
                    let (element, at) = match search::element_type(kind, &written) {
                        Some(element) => (element, at.clone()),
                        None => named(&written)?,
                    };
                    let (ty, call_signature) = match kind == Kind::Go && element == "struct" {
                        true => (anonymous(&at, c.line1), None),
                        false => (self.type_decl(kind, &at, &element)?, Some(link)),
                    };
                    match &found {
                        Some(one) if (&one.ty.path, one.ty.line) != (&ty.path, ty.line) => {
                            return None;
                        }
                        Some(_) => {}
                        None => found = Some(TypeVia { ty, call_signature }),
                    }
                }
                found
            }
            // `const { repo } = this`: the field of what the chain on the right proves.
            search::Value::Field(from, field) if hops > 0 => {
                let (ty, _) = self
                    .chain_type(kind, file, text, b.line1, from, hops - 1)
                    .ok()?;
                let TypeVia { ty, .. } = self
                    .hierarchy(kind, &ty, 0, &mut |t| self.field_type(kind, t, field))
                    .flatten()?;
                Some(TypeVia {
                    ty,
                    call_signature: None,
                })
            }
            search::Value::Member(head, field) => {
                let at = search::Binding {
                    line1: b.line1,
                    value: (**head).clone(),
                };
                let TypeVia { ty, .. } = self.binding_type(kind, file, text, &at, hops)?;
                let TypeVia { ty, .. } = self
                    .hierarchy(kind, &ty, 0, &mut |t| self.field_type(kind, t, field))
                    .flatten()?;
                Some(TypeVia {
                    ty,
                    call_signature: None,
                })
            }
            search::Value::Name(_)
            | search::Value::Element(_)
            | search::Value::Field(..)
            | search::Value::Unknown => None,
        }
    }

    /// What a call of `callee` on `line1` of `file` gives: the class it constructs, or the
    /// declared return type of the function, resolved in the file declaring it. `e.RequestInfo()`
    /// on a receiver whose type is proven is the method of that type, or of one it extends. A
    /// return type is never followed through another call; the receiver may itself come from a
    /// call, and `hops` bounds that, so two locals assigned from each other end. The signature is spelled as the language writes it.
    fn call_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        callee: &str,
        hops: usize,
    ) -> Option<TypeVia> {
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        let (method, receiver) = parts.split_last()?;
        // `super.make()` is the base's `make`, which only `typed_definitions` knows how to find:
        // read as a call on `this`, an override's narrower return type would answer.
        if receiver.first().is_some_and(|r| r == "super") {
            return None;
        }
        let proven = (hops > 0 && !receiver.is_empty())
            .then(|| {
                self.chain_type(kind, file, text, line1, receiver, hops - 1)
                    .ok()
            })
            .flatten();
        let decl = match proven {
            Some((ty, _)) => {
                let found = self.hierarchy(kind, &ty, 0, &mut |t| {
                    let members = self.members_of(kind, t, method);
                    (!members.is_empty()).then_some(members)
                })?;
                <[Hit; 1]>::try_from(found).ok().map(|[hit]| hit)?
            }
            None => {
                // A parameter or a local named like a function or an import is a value: what it
                // returns when called is not what that function declares.
                if hidden(kind, file, text, line1, &parts[0]) {
                    return None;
                }
                self.declaration(kind, file, &parts)?
            }
        };
        if search::declares_type(kind, &decl.text) {
            let ty = Typed {
                name: declared_name(kind, &decl.text, &parts),
                path: decl.path,
                line: decl.line,
            };
            return Some(TypeVia {
                ty,
                call_signature: None,
            });
        }
        let text = self.text_of(&decl.path)?;
        let (written, signature) = match self.returns_of(kind, &decl.path, &text, decl.line)? {
            search::Value::New(t) if kind == Kind::Python => {
                (t.clone(), format!("{callee}() returns {t}()"))
            }
            search::Value::New(t) => (t.clone(), format!("{callee}() returns new {t}()")),
            search::Value::Type(t) => {
                let signature = signature(kind, callee, &t);
                (t, signature)
            }
            _ => return None,
        };
        let ty = self.type_decl(kind, &decl.path, &written)?;
        Some(TypeVia {
            ty,
            call_signature: Some(signature),
        })
    }

    /// The return type the function `callee` of `file` declares, as written, and the file that
    /// writes it. A class, and a function that declares none, give nothing.
    fn declared_return(&self, kind: Kind, file: &Path, callee: &str) -> Option<(String, PathBuf)> {
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        let decl = self.declaration(kind, file, &parts)?;
        let text = self.text_of(&decl.path)?;
        match self.returns_of(kind, &decl.path, &text, decl.line)? {
            search::Value::Type(t) => Some((t, decl.path)),
            _ => None,
        }
    }

    /// The declaration of the type written as `written` in `file`.
    pub(super) fn type_decl(&self, kind: Kind, file: &Path, written: &str) -> Option<Typed> {
        self.type_decl_at(kind, file, written, 0)
    }

    fn type_decl_at(&self, kind: Kind, file: &Path, written: &str, depth: usize) -> Option<Typed> {
        let parts = search::type_path(kind, written)?;
        let Some(decl) = self.declaration(kind, file, &parts) else {
            return self.typedef(kind, file, &parts, depth);
        };
        // Go's `type X = Y` is `Y` itself, methods and fields (#100), where `Y` is a type the
        // project declares; `type X = []Y` and the like stay what their line says.
        // ponytail: eight aliases deep, which also ends a cycle.
        // An alias that methods are declared on, `func (t Twin) Close()`, answers for them under
        // its own name, as before.
        let receiver = |alias: &str| {
            let files = self.package_files(kind, &decl.path);
            let pattern = format!(r"^func\s+\(\s*(?:\w+\s+)?\*?{}\b", regex::escape(alias));
            !self.grep_in(&pattern, &files).is_empty()
        };
        if depth < 8
            && let Some(named) = search::go_alias(kind, &decl.text)
            && !parts.last().is_some_and(|alias| receiver(alias))
            && let Some(ty) = self.type_decl_at(kind, &decl.path, named, depth + 1)
        {
            return Some(ty);
        }
        // The name is the declaration's own: behind `from repos import UserRepository as Users`
        // the members are `UserRepository`'s (#100).
        // So is a TypeScript one, which an `export { Hono as HonoBase }` renames.
        if !search::declares_type(kind, &decl.text) {
            return self.required(kind, file, &parts);
        }
        Some(Typed {
            name: declared_name(kind, &decl.text, &parts),
            path: decl.path,
            line: decl.line,
        })
    }
}

impl App {
    /// Whether the JSDoc type `t` of `file` names a type the project declares, itself or as the
    /// element of `T[]`: only then does it say more than the code it documents (#347). A
    /// `{Object}` or `{string}` leaves the default value, the body's `return new X()`, to say.
    fn jsdoc_resolves(&self, kind: Kind, file: &Path, t: &str) -> bool {
        let element = search::element_type(kind, t);
        self.type_decl(kind, file, element.as_deref().unwrap_or(t))
            .is_some()
    }

    /// The declarations of `name` that `line1` of `text`, the text of `file`, reads
    /// ([`search::bindings`]), each typed by the JSDoc a JavaScript file writes for it, as an
    /// annotation types it in TypeScript (#347), when that type resolves.
    fn bindings_of(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
    ) -> Vec<search::Binding> {
        let mut bindings = search::bindings(kind, text, line1, name);
        if search::reads_jsdoc(kind, file) {
            for b in &mut bindings {
                if let Some(t) = search::jsdoc_binding(text, b.line1, name)
                    .filter(|t| self.jsdoc_resolves(kind, file, t))
                {
                    b.value = search::Value::Type(t);
                }
            }
        }
        bindings
    }

    /// What the function declared on `decl1` of `text`, the text of `file`, returns
    /// ([`search::returns`]). In a JavaScript file its JSDoc `@returns {T}` says it first, when
    /// `T`, or the element of a `T[]`, is a type the project declares (#347).
    fn returns_of(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        decl1: usize,
    ) -> Option<search::Value> {
        let declared = search::reads_jsdoc(kind, file)
            .then(|| search::jsdoc_returns(text, decl1))
            .flatten()
            .filter(|t| self.jsdoc_resolves(kind, file, t));
        match declared {
            Some(t) => Some(search::Value::Type(t)),
            None => search::returns(kind, text, decl1),
        }
    }

    /// The JSDoc `@typedef` of a JavaScript `file` that declares the type `parts` names (#347):
    /// a `@typedef {Object}` with its `@property` lines, or `@typedef {import("./x").Name}`, the
    /// type `Name` of the module an import of `./x` reads. A name the file declares twice is
    /// none.
    fn typedef(&self, kind: Kind, file: &Path, parts: &[String], depth: usize) -> Option<Typed> {
        static IMPORTED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r#"^import\(\s*["']([^"']+)["']\s*\)\.([A-Za-z_$][\w$]*)$"#).unwrap()
        });
        let [name] = parts else {
            return None;
        };
        if !search::reads_jsdoc(kind, file) || depth >= 8 {
            return None;
        }
        let text = self.text_of(file)?;
        let [(line, written)] = &search::jsdoc_typedefs(&text, name)[..] else {
            return None;
        };
        if search::jsdoc_object(written) {
            return Some(Typed {
                name: name.clone(),
                path: file.to_path_buf(),
                line: *line,
            });
        }
        let c = IMPORTED.captures(written)?;
        let import = format!("import {{ {} }} from \"{}\";", &c[2], &c[1]);
        let path = search::imports(kind, &import).pop()?.path;
        let (taken, module) = path.split_last()?;
        let files = search::module_files(kind, &self.root, &self.files, file, module);
        let [module] = &files[..] else {
            return None;
        };
        self.type_decl_at(kind, module, taken, depth + 1)
    }
}

impl App {
    /// The class a JavaScript `file` binds to `parts` by `const X = require("./x")` (#347): what
    /// the module hands out, when it declares a type. It types what a JSDoc type names, and as
    /// any type the file writes, `new X()` and `extends X` too.
    fn required(&self, kind: Kind, file: &Path, parts: &[String]) -> Option<Typed> {
        let [name] = parts else {
            return None;
        };
        if !search::reads_jsdoc(kind, file) {
            return None;
        }
        let path = bound(&search::imports(kind, &self.text_of(file)?), name)?;
        let found = self.imported_definitions(kind, file, name, &[], &path)?;
        let [found] = &found[..] else {
            return None;
        };
        let hit = &found.hit;
        search::declares_type(kind, &hit.text).then(|| Typed {
            name: declared_name(kind, &hit.text, parts),
            path: hit.path.clone(),
            line: hit.line,
        })
    }
}

pub(super) fn anonymous(file: &Path, line: usize) -> Typed {
    Typed {
        name: "struct{\u{2026}}".to_owned(),
        path: file.to_path_buf(),
        line,
    }
}

/// Whether a parameter or a local binds `name` where `line1` of `text` reads it: a value
/// then, whatever function, import or namespace of that name the file can see. In JavaScript a
/// `const X = require("./x")` is an import (#328), so `X.make()` returns what `make` declares
/// (#347); TypeScript reads what `require` gives as `any`.
fn hidden(kind: Kind, file: &Path, text: &str, line1: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let required = Regex::new(&format!(
        r"^\s*(?:(?:export\s+)?(?:const|let|var)\s+)?{}\s*=\s*require\(",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    search::bindings(kind, text, line1, name).iter().any(|b| {
        lines.get(b.line1 - 1).is_some_and(|l| {
            !names_itself(kind, l, name)
                && !(search::reads_jsdoc(kind, file) && required.is_match(l))
        })
    })
}

/// The name of the type the line `decl` declares, reached as `parts`. Python and TypeScript read
/// it off the line: behind `from repos import UserRepository as Users`, or
/// `export { Hono as HonoBase }`, the last part is the alias, and the members are the
/// declaration's (#100).
fn declared_name(kind: Kind, decl: &str, parts: &[String]) -> String {
    search::type_name(kind, decl)
        // `export default class extends Base {` has no name of its own.
        .filter(|n| kind == Kind::Python || kind == Kind::TsJs && n != "extends")
        .or_else(|| parts.last().cloned())
        .unwrap_or_default()
}

/// A Swift type a receiver is proven to be (#384): its name as written, the name
/// [`search::qualified`] gives its members' owner, and for a type the project declares its
/// keyword and the first type its header inherits. A type the project only extends has no
/// keyword.
struct SwiftType {
    name: String,
    owner: String,
    keyword: Option<String>,
    base: Option<String>,
}

impl App {
    /// The declarations of `word` on the Swift receiver `chain` whose type is proven (#384): a
    /// parameter's or a property's annotation, a construction of a type the project declares, a
    /// call's `-> Type`, `self`, a loop over `[Type]`, up to six names long. The member is the
    /// type's own, its extensions', then its superclasses'. On a type the project only extends,
    /// the extensions that declare it, or an empty list for "no definition". `None` when a link
    /// is not proven, or the type declares nothing of the name: the search by name decides.
    pub(super) fn swift_typed(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        chain: &[String],
    ) -> Option<Vec<Candidate>> {
        let cut = self.truncated.get();
        let found = (|| {
            let (written, links) = self.swift_chain(here, here, text, self.line + 1, chain, 1)?;
            let ty = self.swift_type(here, &written)?;
            let mut rows = self.swift_member_rows(here, &ty, word)?;
            // A value reaches the instance members; a `static` one needs the type.
            if rows
                .iter()
                .any(|h| search::swift_static_member(&h.text) == Some(false))
            {
                rows.retain(|h| search::swift_static_member(&h.text) != Some(true));
            }
            let label = links.join(" \u{2192} ");
            Some(
                rows.into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Receiver(label.clone()),
                    })
                    .collect(),
            )
        })();
        // A cut in a grep whose result is dropped says nothing about the list shown in the end.
        self.truncated.set(cut);
        found
    }

    /// The type written for the last of `names` on `line1` of `text`, the text of `file`,
    /// as written, and the links that prove it.
    fn swift_chain(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        names: &[String],
        hops: usize,
    ) -> Option<(String, Vec<String>)> {
        let (mut written, link) = self.swift_value(here, file, text, line1, &names[0], hops)?;
        let shown = |w: &str| search::swift_type_name(w).unwrap_or_else(|| w.to_owned());
        let mut links = vec![link.unwrap_or_else(|| format!("{}: {}", names[0], shown(&written)))];
        for (i, field) in names[1..].iter().enumerate() {
            // ponytail: six names in front of the word, as the other languages read.
            if i == 5 {
                return None;
            }
            let ty = self.swift_type(here, &written)?;
            let (next, link) = self.swift_field(here, &ty, field, hops)?;
            let link = link.unwrap_or_else(|| match i == 0 && names[0] == "self" {
                true => format!("self.{field}: {}", shown(&next)),
                false => format!("{field}: {}", shown(&next)),
            });
            match i == 0 && names[0] == "self" {
                true => links[0] = link,
                false => links.push(link),
            }
            written = next;
        }
        Some((written, links))
    }

    /// The type `name` holds on `line1` of `text`, the text of `file`, as written, with
    /// the signature of the call it came from: `self` is the type around the line; a local or a
    /// parameter what its one binding gives it; else a property of the type around the line,
    /// when nothing in its function may bind the name unread.
    fn swift_value(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        hops: usize,
    ) -> Option<(String, Option<String>)> {
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let around = search::swift_enclosing_type(&lines, &literal, line1);
        if name == "self" {
            let own = search::swift_type_header(lines[around? - 1])?.name;
            return Some((own, None));
        }
        match search::bindings(Kind::Swift, text, line1, name).as_slice() {
            [b] => {
                let given = search::swift_given(lines.get(b.line1 - 1)?, name)?;
                self.swift_given_type(here, file, text, b.line1, given, hops)
            }
            [] => {
                let at = around?;
                // The lines of the function under the type, down to `line1`.
                let mut top = search::swift_scope(&lines, &literal, line1).0;
                while top > at {
                    match search::swift_scope(&lines, &literal, top).0 {
                        up if up > at => top = up,
                        _ => break,
                    }
                }
                if top == 0 || (top..line1).any(|l| search::swift_may_bind(lines[l - 1], name)) {
                    return None;
                }
                let own = search::swift_type_header(lines[at - 1])?.name;
                let ty = self.swift_type(here, &own)?;
                self.swift_field(here, &ty, name, hops)
            }
            _ => None,
        }
    }

    /// What `given` on `line1` of `file` reads as a type, as written. A type named by a
    /// generic parameter in scope there is no type the project declares.
    fn swift_given_type(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        given: search::SwiftGiven,
        hops: usize,
    ) -> Option<(String, Option<String>)> {
        let (written, link) = match given {
            search::SwiftGiven::Type(t) => (t, None),
            search::SwiftGiven::Value(e) => self.swift_expr(here, file, text, line1, &e, hops)?,
            search::SwiftGiven::Element(e) => {
                let (w, link) = self.swift_expr(here, file, text, line1, &e, hops)?;
                (search::swift_element(&w)?, link)
            }
        };
        let name = search::swift_type_name(&written)
            .or_else(|| search::swift_element(&written))
            .unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let mut at = line1;
        while at > 0 {
            if search::swift_generics(lines[at - 1]).contains(&name) {
                return None;
            }
            at = search::swift_scope(&lines, &literal, at).0;
        }
        Some((written, link))
    }

    /// The type the Swift expression `e` on `line1` of `file` gives, as written: a
    /// construction of a type the project declares, the `-> Type` of the one function or method
    /// called, a cast, or the chain of names it is.
    fn swift_expr(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        e: &str,
        hops: usize,
    ) -> Option<(String, Option<String>)> {
        let callee = match search::swift_expr(e)? {
            search::SwiftExpr::Cast(t) => return Some((t, None)),
            search::SwiftExpr::Chain(c) => {
                let names: Vec<String> = c.split('.').map(str::to_owned).collect();
                let (w, _) =
                    self.swift_chain(here, file, text, line1, &names, hops.checked_sub(1)?)?;
                return Some((w, None));
            }
            search::SwiftExpr::Call(callee) => callee,
        };
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        // `Type(…)`, `Type.init(…)`: a type the project declares, or nothing a call reads.
        if let [ty] | [ty, _] = parts.as_slice()
            && (parts.len() == 1 || parts[1] == "init")
            && let Some(t) = self.swift_type(here, ty)
        {
            return t.keyword.is_some().then_some((t.name, None));
        }
        let (method, receiver) = parts.split_last()?;
        let (decl, owner) = match receiver {
            // A function of the file's module, which no local or parameter hides.
            [] => {
                if !search::bindings(Kind::Swift, text, line1, method).is_empty() {
                    return None;
                }
                let pattern = search::def_patterns(Kind::Swift, method).join("|");
                let funcs: Vec<Hit> = self
                    .project_definitions(Kind::Swift, here, method, &pattern)
                    .into_iter()
                    .filter(|h| {
                        h.text.contains("func ")
                            && self.text_of(&h.path).is_some_and(|t| {
                                search::qualified(Kind::Swift, &t, h.line, method)
                                    .is_none_or(|q| q == *method)
                            })
                    })
                    .collect();
                let [decl] = <[Hit; 1]>::try_from(funcs).ok()?;
                (decl, None)
            }
            _ => {
                let (w, _) =
                    self.swift_chain(here, file, text, line1, receiver, hops.checked_sub(1)?)?;
                let ty = self.swift_type(here, &w)?;
                let rows = self.swift_member_rows(here, &ty, method)?;
                let [decl] = <[Hit; 1]>::try_from(rows).ok()?;
                (decl, Some(ty.name))
            }
        };
        if !decl.text.contains("func ") {
            return None;
        }
        let returns = search::swift_returns(&self.text_of(&decl.path)?, decl.line)?;
        let returns = match (returns.as_str(), owner) {
            ("Self", Some(owner)) => owner,
            _ => returns,
        };
        let link = format!("{callee}() -> {returns}");
        Some((returns, Some(link)))
    }

    /// The property `name` of `ty`, or of what it extends, as written, with the signature of the
    /// call it came from: one `let` or `var` line.
    fn swift_field(
        &self,
        here: &Path,
        ty: &SwiftType,
        name: &str,
        hops: usize,
    ) -> Option<(String, Option<String>)> {
        let rows = self.swift_member_rows(here, ty, name)?;
        let [row] = <[Hit; 1]>::try_from(rows).ok()?;
        if search::swift_static_member(&row.text).is_none() || row.text.contains("func ") {
            return None;
        }
        let text = self.text_of(&row.path)?;
        let given = search::swift_given(&row.text, name)?;
        self.swift_given_type(here, &row.path, &text, row.line, given, hops)
    }

    /// The Swift type written as `written`: the one the project declares by that name, not a
    /// protocol or an alias; or, when it declares none and extends it, a type from outside.
    fn swift_type(&self, here: &Path, written: &str) -> Option<SwiftType> {
        let name = search::swift_type_name(written)?;
        let pattern = search::def_patterns(Kind::Swift, &name).join("|");
        let mut decls = Vec::new();
        let mut extended = false;
        for h in self.project_definitions(Kind::Swift, here, &name, &pattern) {
            if !self.in_code(Kind::Swift, &h) {
                continue;
            }
            match search::swift_type_header(&h.text) {
                Some(t) if t.keyword == "extension" => extended |= t.name == name,
                Some(t) if t.name == name => decls.push((h, t.keyword, t.first_inherited)),
                Some(_) => {}
                // `typealias Name`, `associatedtype Name`: no type of its own.
                None if !h.text.contains("func ")
                    && search::swift_static_member(&h.text).is_none()
                    && !search::swift_case(&h.text) =>
                {
                    return None;
                }
                None => {}
            }
        }
        match decls.as_slice() {
            [] if extended => Some(SwiftType {
                owner: name.clone(),
                name,
                keyword: None,
                base: None,
            }),
            [(h, k, b)] if k != "protocol" => {
                let text = self.text_of(&h.path)?;
                Some(SwiftType {
                    owner: search::qualified(Kind::Swift, &text, h.line, &name)
                        .unwrap_or(name.clone()),
                    name,
                    keyword: Some(k.clone()),
                    base: b.clone(),
                })
            }
            _ => None,
        }
    }

    /// The project's declarations of `word` as a member of `ty`: its body's and its extensions',
    /// else, for a class, those of the class it extends, and so on up. `None` when none declares
    /// it and `ty` is declared in the project, whose protocols or bases outside may; an empty
    /// list for a type from outside.
    fn swift_member_rows(&self, here: &Path, ty: &SwiftType, word: &str) -> Option<Vec<Hit>> {
        let pattern = search::def_patterns(Kind::Swift, word).join("|");
        let hits = self.project_definitions(Kind::Swift, here, word, &pattern);
        let mut owner = ty.owner.clone();
        let (mut keyword, mut base) = (ty.keyword.clone(), ty.base.clone());
        let mut seen = HashSet::from([owner.clone()]);
        loop {
            let full = format!("{owner}.{word}");
            let rows: Vec<Hit> = hits
                .iter()
                .filter(|h| {
                    self.text_of(&h.path)
                        .and_then(|t| search::qualified(Kind::Swift, &t, h.line, word))
                        .is_some_and(|q| q == full || q.ends_with(&format!(".{full}")))
                        && !search::swift_extension(&h.text)
                        && self.in_code(Kind::Swift, h)
                })
                .cloned()
                .collect();
            if !rows.is_empty() {
                return Some(rows);
            }
            match (keyword.as_deref(), base) {
                (None, _) => return Some(Vec::new()),
                (Some("class"), Some(next)) if seen.insert(next.clone()) => {
                    let up = self.swift_type(here, &next)?;
                    if up.keyword.as_deref() != Some("class") {
                        return None;
                    }
                    (owner, keyword, base) = (up.owner, up.keyword, up.base);
                }
                _ => return None,
            }
        }
    }
}
