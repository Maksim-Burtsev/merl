use super::{
    App, Candidate, HashSet, Hit, Kind, Path, PathBuf, Reason, Regex, Typed, bound, names_itself,
    search, signature,
};

struct TypeVia {
    ty: Typed,
    call_signature: Option<String>,
}
struct SwiftChain {
    written: String,
    links: Vec<String>,
}
struct WrittenVia {
    written: String,
    call_signature: Option<String>,
}
impl WrittenVia {
    fn plain(written: String) -> Self {
        Self {
            written,
            call_signature: None,
        }
    }
}
struct ProvenType {
    ty: Typed,
    links: Vec<String>,
}
#[derive(PartialEq, Eq)]
enum Ancestry {
    ReachesOutsideTheProject,
    ProjectOnly,
}
enum DeclaredField {
    Typed(TypeVia),
    UnreadableOrDisagreeing,
}
impl DeclaredField {
    fn typed(self) -> Option<TypeVia> {
        match self {
            Self::Typed(via) => Some(via),
            Self::UnreadableOrDisagreeing => None,
        }
    }
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
        let ProvenType { ty, links } = self.receiver(kind, here, chain, head)?;
        let declared = self.hierarchy(kind, &ty, 0, &mut |t| self.instance_member(kind, t, word));
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
        if self.on_template(here) {
            Err(first.cloned().unwrap_or_default())
        } else {
            Ok((self.buf.lines.join("\n"), self.line + 1))
        }
    }

    fn receiver(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        head: Option<&search::CallHead>,
    ) -> Result<ProvenType, String> {
        let (text, line) = self.scope(here, chain.first())?;
        match head {
            Some(search::CallHead {
                call_without_arguments: call,
                value,
                fields_to_word: fields,
            }) => {
                let value = value.clone();
                let cast = matches!(value, search::Value::Type(_) | search::Value::Cast { .. })
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
        if before.is_empty() {
            let text = self.buf.lines.join("\n");
            let bindings = search::bindings(kind, &text, self.line + 1, last);
            Some((here.to_path_buf(), text, bindings, Vec::new()))
        } else {
            let ProvenType { ty, links } = self.receiver(kind, here, before, None).ok()?;
            let found = self.hierarchy(kind, &ty, 0, &mut |t| {
                let text = self.text_of(&t.path)?;
                let bindings = search::field_bindings(kind, &text, t.line, last);
                (!bindings.is_empty()).then(|| (t.path.clone(), text, bindings))
            })?;
            Some((found.0, found.1, found.2, links))
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
            line1: line,
            byte_col: None,
            deleted: None,
        };
        Some(vec![Candidate {
            hit,
            reason: Reason::Receiver(links.join(" \u{2192} ")),
        }])
    }

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
        let ProvenType { ty, .. } = self.receiver(kind, here, chain, head).ok()?;
        (self.python_ancestry(&ty, 0)? == Ancestry::ReachesOutsideTheProject).then_some(ty)
    }

    fn python_ancestry(&self, ty: &Typed, depth: usize) -> Option<Ancestry> {
        let kind = Kind::Python;
        let text = self.text_of(&ty.path).filter(|_| depth < 8)?;
        let imports = search::imports(kind, &text);
        let mut outside = false;
        for base in search::bases(kind, &text, ty.line) {
            let parts = search::type_path(kind, &base)?;
            let (name, chain) = parts.split_last()?;
            if matches!(name.as_str(), "object" | "Generic" | "Protocol" | "ABC") {
                continue;
            }
            if let Some(base) = self.type_decl(kind, &ty.path, &base) {
                outside |= base.path.is_absolute()
                    || self.python_ancestry(&base, depth + 1)?
                        == Ancestry::ReachesOutsideTheProject;
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
        Some(if outside {
            Ancestry::ReachesOutsideTheProject
        } else {
            Ancestry::ProjectOnly
        })
    }

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
            header = header && !l.split('#').next().unwrap_or(l).contains(':');
        }
        false
    }

    fn chain_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        chain: &[String],
        hops: usize,
    ) -> Result<ProvenType, String> {
        let start = self
            .value_type(kind, file, text, line1, &chain[0], hops)
            .ok_or_else(|| chain[0].clone())?;
        self.follow(kind, start, &chain[0], true, &chain[1..])
    }

    fn follow(
        &self,
        kind: Kind,
        start: TypeVia,
        name: &str,
        merge: bool,
        fields: &[String],
    ) -> Result<ProvenType, String> {
        let TypeVia {
            mut ty,
            call_signature,
        } = start;
        let mut links = vec![call_signature.unwrap_or_else(|| format!("{name}: {}", ty.name))];
        for (i, field) in fields.iter().enumerate() {
            if i == 5 {
                return Err(field.clone());
            }
            let TypeVia {
                ty: next,
                call_signature,
            } = self
                .hierarchy(kind, &ty, 0, &mut |t| self.field_type(kind, t, field))
                .and_then(DeclaredField::typed)
                .ok_or_else(|| field.clone())?;
            let first = i == 0 && merge;
            let label = if first {
                format!("{name}.{field}")
            } else {
                field.clone()
            };
            let link = call_signature.unwrap_or_else(|| format!("{label}: {}", next.name));
            if first {
                links[0] = link;
            } else {
                links.push(link);
            }
            ty = next;
        }
        Ok(ProvenType { ty, links })
    }

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

    fn field_type(&self, kind: Kind, ty: &Typed, field: &str) -> Option<DeclaredField> {
        let text = self.text_of(&ty.path)?;
        let bindings = search::field_bindings(kind, &text, ty.line, field);
        (!bindings.is_empty()).then(|| match self.agree(kind, &ty.path, &text, &bindings, 1) {
            Some(via) => DeclaredField::Typed(via),
            None => DeclaredField::UnreadableOrDisagreeing,
        })
    }

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
            search::Value::Cast {
                callee,
                written_type: t,
            } => {
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
            search::Value::Element(n) if hops > 0 => {
                self.element_binding_type(kind, file, text, b.line1, n, hops)
            }
            search::Value::Field { chain: from, field } if hops > 0 => {
                let ProvenType { ty, .. } = self
                    .chain_type(kind, file, text, b.line1, from, hops - 1)
                    .ok()?;
                let TypeVia { ty, .. } = self
                    .hierarchy(kind, &ty, 0, &mut |t| self.field_type(kind, t, field))
                    .and_then(DeclaredField::typed)?;
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
                    .and_then(DeclaredField::typed)?;
                Some(TypeVia {
                    ty,
                    call_signature: None,
                })
            }
            search::Value::Name(_)
            | search::Value::Element(_)
            | search::Value::Field { .. }
            | search::Value::Unknown => None,
        }
    }

    fn element_binding_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line1: usize,
        n: &str,
        hops: usize,
    ) -> Option<TypeVia> {
        let (file, text, bindings) = match n.rsplit_once('.') {
            None => (
                file.to_path_buf(),
                text.to_owned(),
                self.bindings_of(kind, file, text, line1, n),
            ),
            Some((chain, last)) => {
                let chain: Vec<String> = chain.split('.').map(str::to_owned).collect();
                let ProvenType { ty, .. } = self
                    .chain_type(kind, file, text, line1, &chain, hops - 1)
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
            let (ty, call_signature) = if kind == Kind::Go && element == "struct" {
                (anonymous(&at, c.line1), None)
            } else {
                (self.type_decl(kind, &at, &element)?, Some(link))
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
        if receiver.first().is_some_and(|r| r == "super") {
            return None;
        }
        let proven = (hops > 0 && !receiver.is_empty())
            .then(|| {
                self.chain_type(kind, file, text, line1, receiver, hops - 1)
                    .ok()
            })
            .flatten();
        let decl = if let Some(ProvenType { ty, .. }) = proven {
            let found = self.hierarchy(kind, &ty, 0, &mut |t| {
                let members = self.members_of(kind, t, method);
                (!members.is_empty()).then_some(members)
            })?;
            <[Hit; 1]>::try_from(found).ok().map(|[hit]| hit)?
        } else {
            if hidden(kind, file, text, line1, &parts[0]) {
                return None;
            }
            self.declaration(kind, file, &parts)?
        };
        if search::declares_type(kind, &decl.text) {
            let ty = Typed {
                name: declared_name(kind, &decl.text, &parts),
                path: decl.path,
                line: decl.line1,
            };
            return Some(TypeVia {
                ty,
                call_signature: None,
            });
        }
        let text = self.text_of(&decl.path)?;
        let (written, signature) = match self.returns_of(kind, &decl.path, &text, decl.line1)? {
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

    fn declared_return(&self, kind: Kind, file: &Path, callee: &str) -> Option<(String, PathBuf)> {
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        let decl = self.declaration(kind, file, &parts)?;
        let text = self.text_of(&decl.path)?;
        match self.returns_of(kind, &decl.path, &text, decl.line1)? {
            search::Value::Type(t) => Some((t, decl.path)),
            _ => None,
        }
    }

    pub(super) fn type_decl(&self, kind: Kind, file: &Path, written: &str) -> Option<Typed> {
        self.type_decl_at(kind, file, written, 0)
    }

    fn type_decl_at(&self, kind: Kind, file: &Path, written: &str, depth: usize) -> Option<Typed> {
        let parts = search::type_path(kind, written)?;
        let Some(decl) = self.declaration(kind, file, &parts) else {
            return self.typedef(kind, file, &parts, depth);
        };
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
        if !search::declares_type(kind, &decl.text) && !js_type(kind, &decl.path, &decl.text) {
            return self.required(kind, file, &parts);
        }
        Some(Typed {
            name: declared_name(kind, &decl.text, &parts),
            path: decl.path,
            line: decl.line1,
        })
    }
}

impl App {
    fn jsdoc_resolves(&self, kind: Kind, file: &Path, t: &str) -> bool {
        let element = search::element_type(kind, t);
        self.type_decl(kind, file, element.as_deref().unwrap_or(t))
            .is_some()
    }

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

    fn typedef(&self, kind: Kind, file: &Path, parts: &[String], depth: usize) -> Option<Typed> {
        static IMPORTED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r#"^import\(\s*["']([^"']+)["']\s*\)\.([A-Za-z_$][\w$]*)$"#)
                .expect("the regex is a literal")
        });
        let [name] = parts else {
            return None;
        };
        if !search::reads_jsdoc(kind, file) || depth >= 8 {
            return None;
        }
        let text = self.text_of(file)?;
        let [
            search::JsdocTypedef {
                line1: line,
                written_type: written,
            },
        ] = &search::jsdoc_typedefs(&text, name)[..]
        else {
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
        (search::declares_type(kind, &hit.text) || js_type(kind, &hit.path, &hit.text)).then(|| {
            Typed {
                name: declared_name(kind, &hit.text, parts),
                path: hit.path.clone(),
                line: hit.line1,
            }
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

fn hidden(kind: Kind, file: &Path, text: &str, line1: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let required = Regex::new(&format!(
        r"^\s*(?:(?:export\s+)?(?:const|let|var)\s+)?{}\s*=\s*require\(",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    search::bindings(kind, text, line1, name).iter().any(|b| {
        lines.get(b.line1 - 1).is_some_and(|l| {
            !(names_itself(kind, l, name)
                || search::reads_jsdoc(kind, file) && required.is_match(l))
        })
    })
}

fn js_type(kind: Kind, file: &Path, line: &str) -> bool {
    search::reads_jsdoc(kind, file)
        && (search::js_constructor(line).is_some() || search::js_exported_class(line))
}

fn declared_name(kind: Kind, decl: &str, parts: &[String]) -> String {
    search::type_name(kind, decl)
        .filter(|_| matches!(kind, Kind::Python | Kind::TsJs))
        .or_else(|| parts.last().cloned())
        .unwrap_or_default()
}

struct SwiftType {
    name: String,
    owner: String,
    keyword: Option<String>,
    base: Option<String>,
}

impl App {
    pub(super) fn swift_typed(
        &self,
        here: &Path,
        text: &str,
        word: &str,
        chain: &[String],
    ) -> Option<Vec<Candidate>> {
        let cut = self.truncated.get();
        let found = (|| {
            let SwiftChain { written, links } =
                self.swift_chain(here, here, text, self.line + 1, chain, 1)?;
            let label = links.join(" \u{2192} ");
            let Some(ty) = self.swift_type(here, &written) else {
                let w = written.trim();
                let w = w
                    .strip_prefix("any ")
                    .or_else(|| w.strip_prefix("some "))
                    .unwrap_or(w);
                let a_type = search::bindings(Kind::Swift, text, self.line + 1, &chain[0])
                    .iter()
                    .any(|b| {
                        text.lines()
                            .nth(b.line1 - 1)
                            .is_some_and(|l| search::swift_type_header(l).is_some())
                    });
                let p = search::swift_type_name(w)
                    .filter(|p| !a_type && self.swift_protocol(here, p))?;
                let mut rows = self.swift_protocol_rows(here, &p, word, true)?;
                if rows.is_empty() {
                    rows = self.swift_protocol_rows(here, &p, word, false)?;
                }
                return (!rows.is_empty()).then(|| {
                    rows.into_iter()
                        .map(|hit| Candidate {
                            hit,
                            reason: Reason::Receiver(label.clone()),
                        })
                        .collect()
                });
            };
            let mut rows = self.swift_member_rows(here, &ty, word)?;
            if rows
                .iter()
                .any(|h| search::swift_member_kind(&h.text) == Some(search::SwiftMember::Instance))
            {
                rows.retain(|h| {
                    search::swift_member_kind(&h.text) != Some(search::SwiftMember::StaticOrClass)
                });
            }
            Some(
                rows.into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Receiver(label.clone()),
                    })
                    .collect(),
            )
        })();
        self.truncated.set(cut);
        found
    }

    fn swift_chain(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        names: &[String],
        hops: usize,
    ) -> Option<SwiftChain> {
        let WrittenVia {
            mut written,
            call_signature: link,
        } = self.swift_value(here, file, text, line1, &names[0], hops)?;
        let shown = |w: &str| search::swift_type_name(w).unwrap_or_else(|| w.to_owned());
        let mut links = vec![link.unwrap_or_else(|| format!("{}: {}", names[0], shown(&written)))];
        for (i, field) in names[1..].iter().enumerate() {
            if i == 5 {
                return None;
            }
            let ty = self.swift_type(here, &written)?;
            let WrittenVia {
                written: next,
                call_signature: link,
            } = self.swift_field(here, &ty, field, hops)?;
            let link = link.unwrap_or_else(|| {
                if i == 0 && names[0] == "self" {
                    format!("self.{field}: {}", shown(&next))
                } else {
                    format!("{field}: {}", shown(&next))
                }
            });
            if i == 0 && names[0] == "self" {
                links[0] = link;
            } else {
                links.push(link);
            }
            written = next;
        }
        Some(SwiftChain { written, links })
    }

    fn swift_value(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        hops: usize,
    ) -> Option<WrittenVia> {
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let around = search::swift_enclosing_type(&lines, &literal, line1);
        if name == "self" {
            let own = search::swift_type_header(lines[around? - 1])?.name;
            return Some(WrittenVia::plain(own));
        }
        match search::bindings(Kind::Swift, text, line1, name).as_slice() {
            [b] => {
                let given = search::swift_given(lines.get(b.line1 - 1)?, name)?;
                self.swift_given_type(here, file, text, b.line1, given, hops)
            }
            [] => {
                let at = around?;
                let mut top = search::swift_scope(&lines, &literal, line1).header_line1;
                while top > at {
                    match search::swift_scope(&lines, &literal, top).header_line1 {
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

    fn swift_given_type(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        given: search::SwiftGiven,
        hops: usize,
    ) -> Option<WrittenVia> {
        let via = match given {
            search::SwiftGiven::Type(t) => WrittenVia::plain(t),
            search::SwiftGiven::Value(e) => self.swift_expr(here, file, text, line1, &e, hops)?,
            search::SwiftGiven::Element(e) => {
                let collection = self.swift_expr(here, file, text, line1, &e, hops)?;
                WrittenVia {
                    written: search::swift_element(&collection.written)?,
                    call_signature: collection.call_signature,
                }
            }
        };
        let name = search::swift_type_name(&via.written)
            .or_else(|| search::swift_element(&via.written))
            .unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Swift, text);
        let mut at = line1;
        while at > 0 {
            if search::swift_generics(lines[at - 1]).contains(&name) {
                return None;
            }
            at = search::swift_scope(&lines, &literal, at).header_line1;
        }
        Some(via)
    }

    fn swift_expr(
        &self,
        here: &Path,
        file: &Path,
        text: &str,
        line1: usize,
        e: &str,
        hops: usize,
    ) -> Option<WrittenVia> {
        let callee = match search::swift_expr(e)? {
            search::SwiftExpr::Cast(t) => return Some(WrittenVia::plain(t)),
            search::SwiftExpr::Chain(c) => {
                let names: Vec<String> = c.split('.').map(str::to_owned).collect();
                let chain =
                    self.swift_chain(here, file, text, line1, &names, hops.checked_sub(1)?)?;
                return Some(WrittenVia::plain(chain.written));
            }
            search::SwiftExpr::Call(callee) => callee,
        };
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        if let [ty] | [ty, _] = parts.as_slice()
            && (parts.len() == 1 || parts[1] == "init")
            && let Some(t) = self.swift_type(here, ty)
        {
            return t.keyword.is_some().then_some(WrittenVia::plain(t.name));
        }
        let (method, receiver) = parts.split_last()?;
        let (decl, owner) = if receiver.is_empty() {
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
                            search::qualified(Kind::Swift, &t, h.line1, method)
                                .is_none_or(|q| q == *method)
                        })
                })
                .collect();
            let [decl] = <[Hit; 1]>::try_from(funcs).ok()?;
            (decl, None)
        } else {
            let chain =
                self.swift_chain(here, file, text, line1, receiver, hops.checked_sub(1)?)?;
            let ty = self.swift_type(here, &chain.written)?;
            let rows = self.swift_member_rows(here, &ty, method)?;
            let [decl] = <[Hit; 1]>::try_from(rows).ok()?;
            (decl, Some(ty.name))
        };
        if !decl.text.contains("func ") {
            return None;
        }
        let returns = search::swift_returns(&self.text_of(&decl.path)?, decl.line1)?;
        let returns = match (returns.as_str(), owner) {
            ("Self", Some(owner)) => owner,
            _ => returns,
        };
        let link = format!("{callee}() -> {returns}");
        Some(WrittenVia {
            written: returns,
            call_signature: Some(link),
        })
    }

    fn swift_field(
        &self,
        here: &Path,
        ty: &SwiftType,
        name: &str,
        hops: usize,
    ) -> Option<WrittenVia> {
        let rows = self.swift_member_rows(here, ty, name)?;
        let [row] = <[Hit; 1]>::try_from(rows).ok()?;
        if search::swift_member_kind(&row.text).is_none() || row.text.contains("func ") {
            return None;
        }
        let text = self.text_of(&row.path)?;
        let given = search::swift_given(&row.text, name)?;
        self.swift_given_type(here, &row.path, &text, row.line1, given, hops)
    }

    #[expect(
        clippy::match_same_arms,
        reason = "a guarded arm sits between the two; merging them would move one past it"
    )]
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
                None if !h.text.contains("func ")
                    && search::swift_member_kind(&h.text).is_none()
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
                    owner: search::qualified(Kind::Swift, &text, h.line1, &name)
                        .unwrap_or_else(|| name.clone()),
                    name,
                    keyword: Some(k.clone()),
                    base: b.clone(),
                })
            }
            _ => None,
        }
    }

    fn swift_member_rows(&self, here: &Path, ty: &SwiftType, word: &str) -> Option<Vec<Hit>> {
        let pattern = search::def_patterns(Kind::Swift, word).join("|");
        let hits = self.project_definitions(Kind::Swift, here, word, &pattern);
        let mut owner = ty.owner.clone();
        let (mut keyword, mut base) = (ty.keyword.clone(), ty.base.clone());
        let mut seen = HashSet::from([owner.clone()]);
        let mut walked = vec![ty.name.clone()];
        loop {
            let full = format!("{owner}.{word}");
            let rows: Vec<Hit> = hits
                .iter()
                .filter(|h| {
                    self.text_of(&h.path)
                        .and_then(|t| search::qualified(Kind::Swift, &t, h.line1, word))
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
                    match self.swift_type(here, &next) {
                        Some(up) if up.keyword.as_deref() == Some("class") => {
                            walked.push(up.name);
                            (owner, keyword, base) = (up.owner, up.keyword, up.base);
                        }
                        None if self.swift_protocol(here, &next) => {
                            return self.swift_defaults(here, &walked, word);
                        }
                        _ => return None,
                    }
                }
                (Some("class"), Some(_)) => return None,
                (Some(_), _) => return self.swift_defaults(here, &walked, word),
            }
        }
    }

    pub(super) fn swift_self_default(
        &self,
        here: &Path,
        own: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let cut = self.truncated.get();
        let rows = self
            .swift_type(here, own)
            .and_then(|ty| self.swift_member_rows(here, &ty, word))
            .filter(|rows| !rows.is_empty());
        self.truncated.set(cut);
        Some(
            rows?
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(format!("self: {own}")),
                })
                .collect(),
        )
    }

    fn swift_protocol(&self, here: &Path, name: &str) -> bool {
        let pattern = search::def_patterns(Kind::Swift, name).join("|");
        let decls: Vec<String> = self
            .project_definitions(Kind::Swift, here, name, &pattern)
            .iter()
            .filter(|h| self.in_code(Kind::Swift, h))
            .filter_map(|h| search::swift_type_header(&h.text))
            .filter(|t| t.name == name && t.keyword != "extension")
            .map(|t| t.keyword)
            .collect();
        matches!(decls.as_slice(), [k] if k == "protocol")
    }

    fn swift_defaults(&self, here: &Path, types: &[String], word: &str) -> Option<Vec<Hit>> {
        let mut protocols: Vec<String> = Vec::new();
        for t in types {
            let pattern = search::def_patterns(Kind::Swift, t).join("|");
            for h in self.project_definitions(Kind::Swift, here, t, &pattern) {
                let Some(head) = search::swift_type_header(&h.text)
                    .filter(|head| head.name == *t && self.in_code(Kind::Swift, &h))
                else {
                    continue;
                };
                let inherited = search::swift_inherited(&h.text);
                if head.where_constrained && !inherited.is_empty() {
                    return None;
                }
                protocols.extend(inherited);
            }
        }
        protocols.sort();
        protocols.dedup();
        let mut rows: Vec<Hit> = Vec::new();
        for p in protocols.iter().filter(|p| self.swift_protocol(here, p)) {
            rows.extend(self.swift_protocol_rows(here, p, word, false)?);
        }
        (!rows.is_empty()).then_some(rows)
    }

    fn swift_protocol_rows(
        &self,
        here: &Path,
        protocol: &str,
        word: &str,
        requirement: bool,
    ) -> Option<Vec<Hit>> {
        let pattern = search::def_patterns(Kind::Swift, word).join("|");
        let full = format!("{protocol}.{word}");
        let mut rows = Vec::new();
        for h in self.project_definitions(Kind::Swift, here, word, &pattern) {
            let text = self.text_of(&h.path)?;
            let named = search::qualified(Kind::Swift, &text, h.line1, word)
                .is_some_and(|q| q == full || q.ends_with(&format!(".{full}")));
            if !named || !self.in_code(Kind::Swift, &h) {
                continue;
            }
            let lines: Vec<&str> = text.lines().collect();
            let literal = search::literal_lines(Kind::Swift, &text);
            let at = search::swift_enclosing_type(&lines, &literal, h.line1)?;
            let head = search::swift_type_header(lines[at - 1])?;
            let constrained = head.where_constrained
                || lines
                    .get(at)
                    .is_some_and(|l| l.trim_start().starts_with("where "));
            match head.keyword.as_str() {
                "protocol" if requirement => rows.push(h),
                "extension" if !requirement && constrained => return None,
                "extension" if !requirement => rows.push(h),
                _ => {}
            }
        }
        Some(rows)
    }
}
