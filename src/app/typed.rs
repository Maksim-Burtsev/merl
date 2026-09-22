//! The type of a receiver: what a value was declared, assigned or returned as.

use super::*;

impl App {
    /// The declarations of `word` in the type of the receiver `chain`, `x.f.g…`, when every link is
    /// proven: every declaration of `x` in scope reads the same type (through one call's return
    /// type at most), each field is declared in the type before it or one that type extends or
    /// embeds, and each type is declared once where the file that names it can see it
    /// ([`App::declaration`]). The member — a method, else a field — is looked for in the last
    /// type, then in the types it extends or embeds. `Err` names the first name that is not
    /// proven; an empty list is a type without the member. Both leave the word to the search by
    /// name.
    pub(super) fn typed_definitions(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        chain: &[String],
        head: Option<&(String, search::Value, Vec<String>)>,
    ) -> Result<Vec<Candidate>, String> {
        let text = self.buf.lines.join("\n");
        let line = self.line + 1;
        // `super().m()` / `super.m()` is `self` / `this` with the walk started one level up.
        if chain.first().is_some_and(|f| f == "super") {
            let [_] = chain else {
                return Err(chain[1].clone());
            };
            let (ty, _) = self
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
        let (ty, links) = match head {
            // The chain hangs off a call: `make_uow().users.word`.
            Some((call, value, fields)) => {
                let value = value.clone();
                // A cast is its own link, as written: `via (repo as UserRepository)`.
                let cast = matches!(value, search::Value::Type(_) | search::Value::Cast(..))
                    .then(|| call.clone());
                let (ty, link) = self
                    .binding_type(kind, here, &text, &search::Binding { line, value }, 1)
                    .ok_or_else(|| call.clone())?;
                let start = (ty, link.or(cast));
                self.follow(kind, start, call, false, fields)?
            }
            None => self.chain_type(kind, here, &text, line, chain, 1)?,
        };
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

    /// `word` as the class `chain` names declares it for the class itself: a method, else a line
    /// of the class body (`CONST = 1`, `RED = 1` of an `Enum`, a dataclass's `x: int`, a
    /// property), in the class or what [`App::above`] proves over it. An attribute a method
    /// assigns to `self` is an instance's, and no answer here.
    pub(super) fn class_attribute(
        &self,
        kind: Kind,
        here: &Path,
        chain: &[String],
        word: &str,
    ) -> Vec<Candidate> {
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

    /// The type of the chain `x.f.g` on 1-based `line` of `text`, the text of `file`, and the links
    /// that prove it; `Err` names the first name that is not proven.
    fn chain_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line: usize,
        chain: &[String],
        hops: usize,
    ) -> Result<(Typed, Vec<String>), String> {
        let start = self
            .value_type(kind, file, text, line, &chain[0], hops)
            .ok_or_else(|| chain[0].clone())?;
        self.follow(kind, start, &chain[0], true, &chain[1..])
    }

    /// From the type `start` of `name`, each of `fields` in the type before it. What the status
    /// line lists: `repo: UserRepository`, or with `merge` `self.uow: UnitOfWork` for the receiver
    /// and its field, then `users: UserRepository` for each field after them.
    fn follow(
        &self,
        kind: Kind,
        start: (Typed, Option<String>),
        name: &str,
        merge: bool,
        fields: &[String],
    ) -> Result<(Typed, Vec<String>), String> {
        let (mut ty, call) = start;
        let mut links = vec![call.unwrap_or_else(|| format!("{name}: {}", ty.name))];
        for (i, field) in fields.iter().enumerate() {
            // ponytail: six names in front of the word; a longer chain breaks at the seventh.
            if i == 5 {
                return Err(field.clone());
            }
            let (next, call) = self
                .hierarchy(kind, &ty, 0, &mut |t| self.field_type(kind, t, field))
                .flatten()
                .ok_or_else(|| field.clone())?;
            let first = i == 0 && merge;
            let label = match first {
                true => format!("{name}.{field}"),
                false => field.clone(),
            };
            let link = call.unwrap_or_else(|| format!("{label}: {}", next.name));
            if first {
                links[0] = link;
            } else {
                links.push(link);
            }
            ty = next;
        }
        Ok((ty, links))
    }

    /// The type of `name` on 1-based `line` of `text`, the text of `file`: the one type all its
    /// declarations in scope read, with the signature of the call it came from, if any. `hops` is
    /// how many times a declaration may hand over to another name (`self.repo = repo`).
    fn value_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line: usize,
        name: &str,
        hops: usize,
    ) -> Option<(Typed, Option<String>)> {
        let bindings = search::bindings(kind, text, line, name);
        if !bindings.is_empty() || kind != Kind::Go {
            return self.agree(kind, file, text, &bindings, hops);
        }
        // A Go name no scope of the file declares is the package's, declared in any of its
        // files (#100); each is read in the file that writes it, and they have to agree. An
        // empty list is no proof of that: the walk misses locals, so the function around the
        // line must not so much as mention the name, and an import of the file is no variable.
        let imported = search::imports(kind, text).iter().any(|(n, _)| n == name);
        if imported || search::go_may_declare(text, line, name) {
            return None;
        }
        let mut found: Option<(Typed, Option<String>)> = None;
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
                Some((ty, _)) if (&ty.path, ty.line) != (&this.0.path, this.0.line) => return None,
                Some(_) => {}
                None => found = Some(this),
            }
        }
        found
    }

    /// The field `field` as `ty` itself declares it: `None` when it does not, `Some(None)` when
    /// its declarations cannot be read or disagree.
    fn field_type(
        &self,
        kind: Kind,
        ty: &Typed,
        field: &str,
    ) -> Option<Option<(Typed, Option<String>)>> {
        let text = self.text_of(&ty.path)?;
        let bindings = search::field_bindings(kind, &text, ty.line, field);
        (!bindings.is_empty()).then(|| self.agree(kind, &ty.path, &text, &bindings, 1))
    }

    /// The one type every binding reads, or `None` when there is none, one cannot be read or
    /// two disagree.
    fn agree(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        bindings: &[search::Binding],
        hops: usize,
    ) -> Option<(Typed, Option<String>)> {
        let mut found: Option<(Typed, Option<String>)> = None;
        for b in bindings {
            let this = self.binding_type(kind, file, text, b, hops)?;
            match &found {
                Some((ty, _)) if (&ty.path, ty.line) != (&this.0.path, this.0.line) => return None,
                Some(_) => {}
                None => found = Some(this),
            }
        }
        found
    }

    /// The type one binding in `file` reads, and the signature of the call it came through.
    fn binding_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        b: &search::Binding,
        hops: usize,
    ) -> Option<(Typed, Option<String>)> {
        match &b.value {
            search::Value::Type(t) | search::Value::New(t) => {
                // `new api.Tool()` on a parameter `api` names no namespace of the file.
                let path = search::type_path(kind, t).filter(|p| p.len() > 1);
                if path.is_some_and(|p| hidden(kind, text, b.line, &p[0])) {
                    return None;
                }
                self.type_decl(kind, file, t).map(|ty| (ty, None))
            }
            search::Value::Class(line) => {
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
                Some((ty, None))
            }
            search::Value::Call(callee) => self.call_type(kind, file, text, b.line, callee, hops),
            // `cast(T, x)` writes `T`, unless the project declares the `cast` this file calls:
            // that one is a function, with whatever it returns.
            search::Value::Cast(callee, t) => {
                let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
                match self.declaration(kind, file, &parts) {
                    Some(_) => self.call_type(kind, file, text, b.line, callee, hops),
                    None => self.type_decl(kind, file, t).map(|ty| (ty, None)),
                }
            }
            search::Value::Name(n) if hops > 0 => {
                self.value_type(kind, file, text, b.line, n, hops - 1)
            }
            // An element of a collection whose every declaration writes its type: an
            // annotation, or the return type of the function it was assigned from.
            search::Value::Element(n) if hops > 0 => {
                let mut found: Option<(Typed, Option<String>)> = None;
                for c in search::bindings(kind, text, b.line, n) {
                    let (written, at, link) = match &c.value {
                        search::Value::Type(t) => {
                            (t.clone(), file.to_path_buf(), format!("{n}: {}", t.trim()))
                        }
                        search::Value::Call(callee) => {
                            // `api.list()` on a parameter `api` is no function of the file.
                            let first = callee.split('.').next().unwrap_or(callee);
                            if hidden(kind, text, c.line, first) {
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
                    let ty = self.type_decl(kind, &at, &element)?;
                    match &found {
                        Some((one, _)) if (&one.path, one.line) != (&ty.path, ty.line) => {
                            return None;
                        }
                        Some(_) => {}
                        None => found = Some((ty, Some(link))),
                    }
                }
                found
            }
            // `const { repo } = this`: the field of what the chain on the right proves.
            search::Value::Field(from, field) if hops > 0 => {
                let (ty, _) = self
                    .chain_type(kind, file, text, b.line, from, hops - 1)
                    .ok()?;
                let (ty, _) = self
                    .hierarchy(kind, &ty, 0, &mut |t| self.field_type(kind, t, field))
                    .flatten()?;
                Some((ty, None))
            }
            search::Value::Name(_)
            | search::Value::Element(_)
            | search::Value::Field(..)
            | search::Value::Unknown => None,
        }
    }

    /// What a call of `callee` on 1-based `line` of `file` gives: the class it constructs, or the
    /// declared return type of the function, resolved in the file declaring it. `e.RequestInfo()`
    /// on a receiver whose type is proven is the method of that type, or of one it extends. A
    /// return type is never followed through another call; the receiver may itself come from a
    /// call, and `hops` bounds that, so two locals assigned from each other end. The signature is spelled as the language writes it.
    fn call_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        line: usize,
        callee: &str,
        hops: usize,
    ) -> Option<(Typed, Option<String>)> {
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        let (method, receiver) = parts.split_last()?;
        // `super.make()` is the base's `make`, which only `typed_definitions` knows how to find:
        // read as a call on `this`, an override's narrower return type would answer.
        if receiver.first().is_some_and(|r| r == "super") {
            return None;
        }
        let proven = (hops > 0 && !receiver.is_empty())
            .then(|| {
                self.chain_type(kind, file, text, line, receiver, hops - 1)
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
                if hidden(kind, text, line, &parts[0]) {
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
            return Some((ty, None));
        }
        let text = self.text_of(&decl.path)?;
        let (written, signature) = match search::returns(kind, &text, decl.line)? {
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
        Some((ty, Some(signature)))
    }

    /// The return type the function `callee` of `file` declares, as written, and the file that
    /// writes it. A class, and a function that declares none, give nothing.
    fn declared_return(&self, kind: Kind, file: &Path, callee: &str) -> Option<(String, PathBuf)> {
        let parts: Vec<String> = callee.split('.').map(str::to_owned).collect();
        let decl = self.declaration(kind, file, &parts)?;
        let text = self.text_of(&decl.path)?;
        match search::returns(kind, &text, decl.line)? {
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
        let decl = self.declaration(kind, file, &parts)?;
        // Go's `type X = Y` is `Y` itself, methods and fields (#100), where `Y` is a type the
        // project declares; `type X = []Y` and the like stay what their line says.
        // ponytail: eight aliases deep, which also ends a cycle.
        // An alias that methods are declared on, `func (t Twin) Close()`, answers for them under
        // its own name, as before.
        let receiver = |alias: &str| {
            let files = self.package_files(kind, &decl.path);
            let pattern = format!(r"^func\s+\(\s*(?:\w+\s+)?\*?{}\b", regex::escape(alias));
            self.grep(&pattern, false, false, |p| files.iter().any(|f| f == p))
                .is_ok_and(|hits| !hits.is_empty())
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
        search::declares_type(kind, &decl.text).then(|| Typed {
            name: declared_name(kind, &decl.text, &parts),
            path: decl.path,
            line: decl.line,
        })
    }
}

/// Whether a parameter or a local binds `name` where 1-based `line` of `text` reads it: a value
/// then, whatever function, import or namespace of that name the file can see.
fn hidden(kind: Kind, text: &str, line: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    search::bindings(kind, text, line, name).iter().any(|b| {
        lines
            .get(b.line - 1)
            .is_some_and(|l| !names_itself(l, name))
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
