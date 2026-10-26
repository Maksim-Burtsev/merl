use super::{App, Candidate, HashMap, Hit, Kind, Path, PathBuf, Reason, Regex, search};

struct WrittenType {
    resolved: JvmType,
    written: String,
}

struct TypedField {
    hit: Hit,
    written: String,
}

pub(super) enum JvmType {
    Project(Hit),
    Outside(String),
    Unknown,
}

impl App {
    fn jvm_own_type(here: &Path, text: &str, line: usize, name: Option<&str>) -> Option<JvmType> {
        let lines: Vec<&str> = text.lines().collect();
        let decl = search::jvm_enclosing_types(text, line)
            .into_iter()
            .find(|&d| {
                search::jvm_type_name(lines[d - 1]).is_some_and(|n| name.is_none_or(|w| n == w))
            })?;
        Some(JvmType::Project(Hit {
            deleted: None,
            path: here.to_path_buf(),
            line1: decl,
            byte_col: None,
            text: lines[decl - 1].to_owned(),
        }))
    }

    pub(super) fn jvm_typed(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let kotlin = here.extension().is_none_or(|e| e != "java");
        if let ([], Some(cast)) = (chain, search::jvm_cast_receiver(before, kotlin)) {
            let ty = self.jvm_type_at(here, text, &cast);
            return self.jvm_typed_member(here, ty, &cast, word);
        }
        if let ([], Some(made)) = (chain, search::jvm_constructed_receiver(before, kotlin)) {
            return match self.jvm_made(here, text, &made, kotlin) {
                ty @ JvmType::Project(_) => self.jvm_typed_member(here, ty, &made, word),
                _ => None,
            };
        }
        if let ([], false, Some(source)) = (chain, kotlin, search::java_element_access(before)) {
            let element = Self::jvm_element(text, self.line + 1, &source, true)?;
            let ty = self.jvm_type_at(here, text, &element);
            let label = format!("{source}: element {element}");
            return self.jvm_typed_member(here, ty, &label, word);
        }
        let kt = here.extension().is_some_and(|e| e == "kt" || e == "kts");
        let safe: Vec<String>;
        let chain = match chain {
            [] if kotlin => {
                let mut plain = before.replace("?.", ".").replace("!!.", ".");
                if kt && let Some(receiver) = search::kotlin_scope_returned(&plain) {
                    plain = receiver;
                }
                safe = search::qualifier(&plain, plain.len());
                if let ([], true, Some(c)) = (&safe[..], kt, search::kotlin_call_chain(&plain)) {
                    let (ty, links) = self.kotlin_chain_type(here, text, Some(before), &c)?;
                    return self.jvm_typed_member(here, ty, &links.join(" \u{2192} "), word);
                }
                &safe[..]
            }
            [] => {
                safe = search::jvm_getter_chain(before);
                &safe[..]
            }
            _ => chain,
        };
        let upper = |n: &str| n.starts_with(|c: char| c.is_ascii_uppercase());
        if let ([named, s], false) = (chain, kotlin)
            && s == "super"
            && upper(named)
            && Self::jvm_own_type(here, text, self.line + 1, Some(named)).is_none()
        {
            let ty = self.jvm_type_at(here, text, named);
            return self.jvm_typed_member(here, ty, &format!("{named}.super"), word);
        }
        let outer = match chain {
            [o, t, ..] if !kotlin && t == "this" && upper(o) => Some(o.as_str()),
            _ => None,
        };
        let chain = if outer.is_some() { &chain[1..] } else { chain };
        let (first, rest) = chain.split_first()?;
        if chain.len() > 6 {
            return None;
        }
        let line = self.line + 1;
        if first == "super" {
            return rest
                .is_empty()
                .then(|| self.jvm_super(here, text, line, word))
                .flatten();
        }
        let (ty, links) = self.jvm_chain_head(here, text, before, first, outer)?;
        let (ty, links) = self.jvm_follow(ty, links, first, rest)?;
        self.jvm_typed_member(here, ty, &links.join(" \u{2192} "), word)
    }

    fn jvm_follow(
        &self,
        mut ty: JvmType,
        mut links: Vec<String>,
        first: &str,
        rest: &[String],
    ) -> Option<(JvmType, Vec<String>)> {
        for (i, field) in rest.iter().enumerate() {
            let JvmType::Project(decl) = ty else {
                return None;
            };
            let TypedField { hit, written } = match field.strip_suffix("()") {
                Some(call) => self.jvm_call_type(&decl, call)?,
                None => self.jvm_field_type(&decl, field)?,
            };
            let t = self.text_of(&hit.path)?;
            ty = self.jvm_type_at(&hit.path, &t, &written);
            links.push(match (i, first) {
                (0, "this") => format!("this.{field}: {written}"),
                _ => format!("{field}: {written}"),
            });
        }
        Some((ty, links))
    }

    fn kotlin_chain_type(
        &self,
        here: &Path,
        text: &str,
        before: Option<&str>,
        c: &search::KotlinCallChain,
    ) -> Option<(JvmType, Vec<String>)> {
        if c.links.len() > 6 {
            return None;
        }
        let (ty, links) = match before {
            _ if c.constructed => (
                self.jvm_made(here, text, &c.head, true),
                vec![format!("{}()", c.head)],
            ),
            Some(before) => self.jvm_chain_head(here, text, before, &c.head, None)?,
            None => return None,
        };
        self.jvm_follow(ty, links, &c.head, &c.links)
    }

    fn kotlin_initialized(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
    ) -> Option<WrittenType> {
        let at = text.lines().nth(line1.checked_sub(1)?)?;
        if let Some(d) = search::kotlin_destructured(at, name) {
            let source = if d.in_loop {
                let element = Self::jvm_element(text, line1, &d.source, false)?;
                self.jvm_type_at(file, text, &element)
            } else {
                self.jvm_value(file, text, line1, &d.source, true, 1)?
                    .resolved
            };
            let JvmType::Project(decl) = source else {
                return None;
            };
            let t = self.text_of(&decl.path)?;
            let written = search::kotlin_component_type(&t, decl.line1, d.index)?;
            return Some(WrittenType {
                resolved: self.jvm_type_at(&decl.path, &t, &written),
                written,
            });
        }
        let init = search::kotlin_initializer(text, line1, name)?;
        if init.trim() == name
            && search::bindings(Kind::Jvm, text, line1.checked_sub(1)?, name).is_empty()
        {
            let members = search::jvm_enclosing_types(text, line1)
                .into_iter()
                .map(|d| search::jvm_members_of(text, d, name))
                .find(|m| !m.is_empty())?;
            let [m] = members[..] else {
                return None;
            };
            let written = search::jvm_declared_type(text.lines().nth(m - 1)?, name, true)?;
            let resolved = self.jvm_type_at(file, text, &written);
            return Some(WrittenType { resolved, written });
        }
        let chain = search::kotlin_call_chain(&format!("{init}."))?;
        let (resolved, links) = self.kotlin_chain_type(file, text, None, &chain)?;
        let written = links.last()?.rsplit(": ").next()?.to_owned();
        Some(WrittenType { resolved, written })
    }

    fn jvm_chain_head(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        first: &str,
        outer: Option<&str>,
    ) -> Option<(JvmType, Vec<String>)> {
        let kotlin = here.extension().is_none_or(|e| e != "java");
        let kt = here.extension().is_some_and(|e| e == "kt" || e == "kts");
        let line = self.line + 1;
        let labelled = |f: &str| before.trim_end().ends_with(&format!("this@{f}."));
        let own = |name| Self::jvm_own_type(here, text, line, name);
        Some(match first {
            f if labelled(f)
                || (f == "this" && search::jvm_enclosing_types(text, line).is_empty()) =>
            {
                let ext = kt
                    .then(|| search::kotlin_extension_at(text, line, before))
                    .flatten()
                    .filter(|e| f == "this" || e.name == f);
                match ext {
                    Some(ext) => {
                        let ty = self.jvm_type_at(here, text, &ext.receiver);
                        (ty, vec![format!("this: {}", ext.receiver)])
                    }
                    None => (own((f != "this").then_some(f))?, Vec::new()),
                }
            }
            "this" => match kt
                .then(|| search::kotlin_lambda_receiver(text, line, before))
                .flatten()
            {
                Some(receiver) => {
                    let WrittenType { resolved, written } =
                        self.jvm_value(here, text, line, &receiver, kotlin, 1)?;
                    (resolved, vec![format!("this: {written}")])
                }
                None => (own(outer)?, Vec::new()),
            },
            f if f.starts_with(|c: char| c.is_ascii_lowercase()) => {
                let smart = kotlin.then(|| search::jvm_smart_cast(text, line, before, f));
                let narrowed = smart.flatten().or_else(|| {
                    let lambda =
                        kt.then(|| search::kotlin_lambda_collection(text, line, before, f));
                    let lambda = lambda.flatten()?;
                    match lambda.strip_suffix(".entries") {
                        Some(e) => match self.jvm_type_at(here, text, e) {
                            JvmType::Project(h) if h.text.contains("enum class") => {
                                Some(e.to_owned())
                            }
                            _ => None,
                        },
                        None => Self::jvm_element(text, line, &lambda, false),
                    }
                });
                let WrittenType {
                    resolved: ty,
                    written,
                } = if let Some(t) = narrowed {
                    WrittenType {
                        resolved: self.jvm_type_at(here, text, &t),
                        written: t,
                    }
                } else {
                    let subject = kt
                        .then(|| search::kotlin_lambda_subject(text, line, before, f))
                        .flatten();
                    let value = subject.as_deref().unwrap_or(f);
                    self.jvm_value(here, text, line, value, kotlin, 1)?
                };
                (ty, vec![format!("{f}: {written}")])
            }
            _ => return None,
        })
    }

    pub(super) fn kotlin_scope_member(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let line = self.line + 1;
        let receiver = search::kotlin_lambda_receiver(text, line, before)?;
        let WrittenType {
            resolved: JvmType::Project(decl),
            written,
        } = self.jvm_value(here, text, line, &receiver, true, 1)?
        else {
            return None;
        };
        let reason = Reason::Receiver(format!("this: {written}"));
        let found: Vec<Candidate> = (self.jvm_hierarchy_member(&decl, word, 0, &mut Vec::new()))
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: reason.clone(),
            })
            .collect();
        (!found.is_empty()).then_some(found)
    }

    fn jvm_made(&self, here: &Path, text: &str, written: &str, kotlin: bool) -> JvmType {
        if !written.contains('.') {
            let found = self
                .jvm_members(here, text, "", written)
                .or_else(|| self.jvm_imported_all(text, &[], written, false))
                .unwrap_or_default();
            let named =
                |c: &Candidate| search::jvm_type_name(&c.hit.text).as_deref() == Some(written);
            if kotlin && !found.iter().all(named) {
                return JvmType::Unknown;
            }
            let types: Vec<Hit> = found.into_iter().filter(named).map(|c| c.hit).collect();
            if !types.is_empty() {
                return one_type(types);
            }
        }
        self.jvm_type_at(here, text, written)
    }

    fn jvm_super(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let lines: Vec<&str> = text.lines().collect();
        let &decl = search::jvm_enclosing_types(text, line1).first()?;
        search::jvm_type_name(lines.get(decl - 1)?)?;
        let mut found = Vec::new();
        for base in search::jvm_bases(text, decl, search::scala(here)) {
            let JvmType::Project(b) = self.jvm_type_at(here, text, &base) else {
                return None;
            };
            let reason = Reason::Receiver(format!("super: {base}"));
            found.extend(
                self.jvm_hierarchy_member(&b, word, 0, &mut Vec::new())
                    .into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: reason.clone(),
                    }),
            );
        }
        (!found.is_empty()).then_some(found)
    }

    pub(super) fn jvm_owner_type(&self, here: &Path, text: &str, owner: &str) -> Option<Hit> {
        if !owner.starts_with(|c: char| c.is_ascii_uppercase()) {
            return None;
        }
        match self.jvm_type_at(here, text, owner) {
            JvmType::Project(decl) => Some(decl),
            _ => None,
        }
    }

    pub(super) fn jvm_owns(&self, decl: &Hit, hit: &Hit, word: &str) -> bool {
        hit.path == decl.path || self.jvm_receiver_of(hit, word).is_some()
    }

    pub(super) fn jvm_owner_member(
        &self,
        here: &Path,
        decl: Hit,
        owner: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let companion = search::jvm_type_name(&decl.text).map(|n| format!("{n}.Companion"));
        self.jvm_typed_member(here, JvmType::Project(decl), owner, word)
            .or_else(|| {
                let companion = companion?;
                let found: Vec<Candidate> = (self.jvm_extensions(here, word).into_iter())
                    .filter(|h| self.jvm_receiver_of(h, word).as_ref() == Some(&companion))
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Receiver(owner.to_owned()),
                    })
                    .collect();
                (!found.is_empty()).then_some(found)
            })
    }

    fn jvm_typed_member(
        &self,
        here: &Path,
        ty: JvmType,
        label: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let found = match ty {
            JvmType::Unknown => return None,
            JvmType::Outside(name) => self.jvm_outside(here, word, &name)?,
            JvmType::Project(decl) => {
                let mut names = Vec::new();
                let mut hits = self.jvm_hierarchy_member(&decl, word, 0, &mut names);
                if hits.is_empty() {
                    hits = self.jvm_extensions(here, word);
                    hits.retain(|h| {
                        self.jvm_receiver_of(h, word)
                            .is_some_and(|r| names.contains(&r))
                    });
                }
                if hits.is_empty() {
                    return None;
                }
                hits
            }
        };
        Some(
            found
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.to_owned()),
                })
                .collect(),
        )
    }

    pub(super) fn jvm_outside(&self, here: &Path, word: &str, name: &str) -> Option<Vec<Hit>> {
        let extensions = self.jvm_extensions(here, word);
        let (on, off): (Vec<Hit>, Vec<Hit>) = extensions
            .into_iter()
            .partition(|h| self.jvm_receiver_of(h, word).as_deref() == Some(name));
        (off.is_empty() || !on.is_empty()).then_some(on)
    }

    fn jvm_extensions(&self, here: &Path, word: &str) -> Vec<Hit> {
        let pattern = search::def_patterns(Kind::Jvm, word).join("|");
        let cut = self.truncated.get();
        let mut hits = self.project_definitions(Kind::Jvm, here, word, &pattern);
        self.truncated.set(cut);
        hits.retain(|h| self.jvm_receiver_of(h, word).is_some());
        hits
    }

    fn jvm_receiver_of(&self, hit: &Hit, word: &str) -> Option<String> {
        search::jvm_receiver(&hit.text, word).or_else(|| {
            let text = self.text_of(&hit.path)?;
            search::jvm_receiver_at(&text, hit.line1, word)
        })
    }

    fn jvm_value(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
        kotlin: bool,
        hops: usize,
    ) -> Option<WrittenType> {
        let narrowing = if kotlin {
            r"\b{0}\s+(?:!?is|as\??)\s|\bwhen\s*\(\s*{0}\s*\)"
        } else {
            r"\b{0}\s+instanceof\s|\(\s*[A-Z][\w.<>]*\s*\)\s*{0}\b"
        };
        let narrowed = Regex::new(&narrowing.replace("{0}", &regex::escape(name))).ok()?;
        let own_when = kotlin
            .then(|| search::jvm_when_else(text, line1, name))
            .flatten();
        if narrowed
            .find_iter(text)
            .any(|m| text.get(..m.start()).map(|t| t.matches('\n').count() + 1) != own_when)
        {
            return None;
        }
        let lines: Vec<&str> = text.lines().collect();
        let ext = file.extension().and_then(|e| e.to_str());
        let literal_typed = matches!(ext, Some("java" | "kt" | "kts"));
        let bindings: Vec<usize> = search::bindings(Kind::Jvm, text, line1, name)
            .iter()
            .map(|b| b.line1)
            .collect();
        let declared = if bindings.is_empty() {
            let members = search::jvm_enclosing_types(text, line1)
                .into_iter()
                .map(|d| search::jvm_members_of(text, d, name))
                .find(|m| !m.is_empty());
            match members {
                Some(m) => m,
                None if matches!(ext, Some("kt" | "kts")) => {
                    vec![search::kotlin_constructor_parameter(text, line1, name)?]
                }
                None => return None,
            }
        } else {
            bindings
        };
        let mut found: Option<WrittenType> = None;
        for l in declared {
            let at = lines.get(l - 1)?;
            let literal = literal_typed
                .then(|| search::jvm_literal_type(at, name, ext == Some("java")))
                .flatten();
            let declared = search::jvm_declared_type(at, name, kotlin);
            let this = match (declared, literal) {
                (None, Some(t)) => WrittenType {
                    resolved: JvmType::Outside(t.to_owned()),
                    written: t.to_owned(),
                },
                (Some(t), _) => WrittenType {
                    resolved: self.jvm_type_at(file, text, &t),
                    written: t,
                },
                (None, None) if hops > 0 => match search::jvm_assigned_call(at, name) {
                    Some(search::JvmAssignedCall {
                        receiver: recv,
                        method: call,
                    }) => self.jvm_returned(file, text, l, recv.as_deref(), &call, kotlin)?,
                    None if literal_typed && kotlin => {
                        self.kotlin_initialized(file, text, l, name)?
                    }
                    None if ext == Some("java") => self.java_lambda_element(file, text, l, name)?,
                    None => return None,
                },
                (None, None) => return None,
            };
            if found.as_ref().is_some_and(|f| f.written != this.written) {
                return None;
            }
            found = Some(this);
        }
        found
    }

    fn java_lambda_element(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        name: &str,
    ) -> Option<WrittenType> {
        let source = search::java_lambda_source(text, line1, name)?;
        if let Some((owner, method)) = source.reference {
            let JvmType::Project(decl) = self.jvm_type_at(file, text, &owner) else {
                return None;
            };
            let TypedField { hit, written } = self.jvm_call_type(&decl, &method)?;
            if !written.starts_with(|c: char| c.is_ascii_uppercase())
                || !written
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
            {
                return None;
            }
            let t = self.text_of(&hit.path)?;
            let resolved = self.jvm_type_at(&hit.path, &t, &written);
            return matches!(resolved, JvmType::Project(_))
                .then_some(WrittenType { resolved, written });
        }
        let (in_file, written) = match (source.receiver, source.call) {
            (Some(r), None) => {
                let [l] = search::bindings(Kind::Jvm, text, line1, &r)
                    .iter()
                    .map(|b| b.line1)
                    .collect::<Vec<_>>()[..]
                else {
                    return None;
                };
                let at = text.lines().nth(l.checked_sub(1)?)?;
                (file.to_path_buf(), search::java_declared_element(at, &r)?)
            }
            (recv, Some((method, args))) => {
                let owner = match recv.as_deref() {
                    None | Some("this") => Self::jvm_own_type(file, text, line1, None)?,
                    Some(r) => self.jvm_value(file, text, line1, r, false, 0)?.resolved,
                };
                let JvmType::Project(decl) = owner else {
                    return None;
                };
                let found = self.jvm_hierarchy_member(&decl, &method, 0, &mut Vec::new());
                let found = java_shaped(found, |h| h, &method, true);
                if let Some(class) = search::java_class_argument(&args) {
                    let taking: Vec<&Hit> = (found.iter())
                        .filter(|h| search::java_takes_class(&h.text, &method))
                        .collect();
                    let exact = !taking.is_empty()
                        && (taking.iter())
                            .all(|h| search::java_returns_class_element(&h.text, &method));
                    exact.then_some((file.to_path_buf(), class))?
                } else {
                    let [hit] = &found[..] else {
                        return None;
                    };
                    let element = search::java_returned_element(&hit.text, &method)?;
                    (hit.path.clone(), element)
                }
            }
            (None, None) => return None,
        };
        let t = self.text_of(&in_file)?;
        let resolved = self.jvm_type_at(&in_file, &t, &written);
        matches!(resolved, JvmType::Project(_)).then_some(WrittenType { resolved, written })
    }

    fn jvm_element(text: &str, line1: usize, name: &str, java: bool) -> Option<String> {
        let mut declared: Vec<usize> = search::bindings(Kind::Jvm, text, line1, name)
            .iter()
            .map(|b| b.line1)
            .collect();
        if declared.is_empty() {
            declared = search::jvm_enclosing_types(text, line1)
                .into_iter()
                .map(|d| search::jvm_members_of(text, d, name))
                .find(|m| !m.is_empty())?;
        }
        let [l] = declared[..] else {
            return None;
        };
        let line = text.lines().nth(l.checked_sub(1)?)?;
        if java {
            search::java_element_type(line, name)
        } else {
            search::kotlin_element_type(line, name)
        }
    }

    fn jvm_returned(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        recv: Option<&str>,
        call: &str,
        kotlin: bool,
    ) -> Option<WrittenType> {
        let owner = if let Some(r) = recv {
            self.jvm_value(file, text, line1, r, kotlin, 0)?.resolved
        } else {
            let lines: Vec<&str> = text.lines().collect();
            let decl = search::jvm_enclosing_types(text, line1)
                .into_iter()
                .find(|&d| search::jvm_type_name(lines[d - 1]).is_some())?;
            JvmType::Project(Hit {
                deleted: None,
                path: file.to_path_buf(),
                line1: decl,
                byte_col: None,
                text: lines[decl - 1].to_owned(),
            })
        };
        let JvmType::Project(decl) = owner else {
            return None;
        };
        let TypedField { hit: m, written } = self.jvm_call_type(&decl, call)?;
        let t = self.text_of(&m.path)?;
        Some(WrittenType {
            resolved: self.jvm_type_at(&m.path, &t, &written),
            written,
        })
    }

    fn jvm_call_type(&self, decl: &Hit, call: &str) -> Option<TypedField> {
        let found = self.jvm_hierarchy_member(decl, call, 0, &mut Vec::new());
        let [hit] = java_shaped(found, |h| h, call, true).try_into().ok()?;
        let mk = hit.path.extension().is_none_or(|e| e != "java");
        let written = search::jvm_return_type(&hit.text, call, mk).or_else(|| {
            let names = search::jvm_accessor(call)?.fields_it_may_read;
            names
                .iter()
                .find_map(|n| search::jvm_declared_type(&hit.text, n, mk))
        })?;
        Some(TypedField { hit, written })
    }

    fn jvm_field_type(&self, decl: &Hit, field: &str) -> Option<TypedField> {
        let mut names = Vec::new();
        let found = self.jvm_hierarchy_member(decl, field, 0, &mut names);
        let [hit] = java_shaped(found, |h| h, field, false).try_into().ok()?;
        let kotlin = hit.path.extension().is_none_or(|e| e != "java");
        let written = search::jvm_declared_type(&hit.text, field, kotlin)?;
        Some(TypedField { hit, written })
    }

    pub(super) fn jvm_hierarchy_member(
        &self,
        decl: &Hit,
        word: &str,
        depth: usize,
        walked: &mut Vec<String>,
    ) -> Vec<Hit> {
        let Some(name) = search::jvm_type_name(&decl.text) else {
            return Vec::new();
        };
        if depth > 8 || walked.contains(&name) {
            return Vec::new();
        }
        walked.push(name);
        let Some(text) = self.text_of(&decl.path) else {
            return Vec::new();
        };
        let lines: Vec<&str> = text.lines().collect();
        let mut found = search::jvm_members_of(&text, decl.line1, word);
        if found.is_empty() {
            found = search::jvm_lombok_fields(&text, decl.line1, word);
        }
        found.sort_unstable();
        found.dedup();
        if !found.is_empty() {
            return found
                .into_iter()
                .map(|line| Hit {
                    deleted: None,
                    path: decl.path.clone(),
                    line1: line,
                    byte_col: None,
                    text: lines[line - 1].to_owned(),
                })
                .collect();
        }
        let mut out = Vec::new();
        let mut known = true;
        for base in search::jvm_bases(&text, decl.line1, search::scala(&decl.path)) {
            match self.jvm_type_at(&decl.path, &text, &base) {
                JvmType::Project(b) => {
                    out.extend(self.jvm_hierarchy_member(&b, word, depth + 1, walked));
                }
                _ => known = false,
            }
        }
        if out.is_empty() && known {
            for base in search::java_implements(&text, decl.line1) {
                if let JvmType::Project(b) = self.jvm_type_at(&decl.path, &text, &base) {
                    out.extend(self.jvm_hierarchy_member(&b, word, depth + 1, walked));
                }
            }
        }
        out
    }

    pub(super) fn jvm_type_at(&self, file: &Path, text: &str, written: &str) -> JvmType {
        if written.starts_with(|c: char| c.is_ascii_lowercase())
            && written.contains('.')
            && let Some(packages) = self.jvm_packages()
        {
            let path: Vec<String> = written.split('.').map(str::to_owned).collect();
            if let Some(ty) = self.jvm_in_package(&packages, &path) {
                return ty;
            }
        }
        if let Some((outer, inner)) = written.rsplit_once('.') {
            return match self.jvm_type_at(file, text, outer) {
                JvmType::Project(decl) => self.jvm_nested(&decl, inner),
                JvmType::Outside(_) => JvmType::Outside(written.to_owned()),
                JvmType::Unknown => JvmType::Unknown,
            };
        }
        if search::jvm_type_parameter(text, written)
            || (search::scala(file) && search::scala_type_parameter(text, written))
        {
            return JvmType::Unknown;
        }
        let is_type = |h: &Hit| search::jvm_type_name(&h.text).as_deref() == Some(written);
        let imports = search::jvm_imports(text);
        if let Some(search::JvmImport { path, .. }) =
            imports.iter().find(|i| i.bound_name == written)
        {
            let Some(packages) = self.jvm_packages() else {
                return JvmType::Unknown;
            };
            return self
                .jvm_in_package(&packages, path)
                .unwrap_or_else(|| JvmType::Outside(written.to_owned()));
        }
        let pattern = search::def_patterns(Kind::Jvm, written).join("|");
        let cut = self.truncated.get();
        let mut hits = self.project_definitions(Kind::Jvm, file, written, &pattern);
        hits.retain(|h| self.jvm_code(h));
        let full = self.truncated.get() && !cut;
        self.truncated.set(cut);
        if full {
            return JvmType::Unknown;
        }
        if hits.is_empty() {
            return JvmType::Outside(written.to_owned());
        }
        let types: Vec<Hit> = hits.iter().filter(|h| is_type(h)).cloned().collect();
        let others = hits
            .iter()
            .any(|h| !is_type(h) && !types.iter().any(|t| t.path == h.path));
        if others {
            return JvmType::Unknown;
        }
        if types.len() < 2 {
            return one_type(types);
        }
        let mine: Vec<Hit> = types.iter().filter(|h| h.path == file).cloned().collect();
        if !mine.is_empty() {
            return one_type(mine);
        }
        let package = search::jvm_package(text);
        let same: Vec<Hit> = types
            .into_iter()
            .filter(|h| {
                package.is_some()
                    && self
                        .text_of(&h.path)
                        .is_some_and(|t| search::jvm_package(&t) == package)
            })
            .collect();
        one_type(same)
    }

    fn jvm_code(&self, h: &Hit) -> bool {
        self.text_of(&h.path).is_some_and(|t| {
            !search::literal_lines(Kind::Jvm, &t)
                .get(h.line1 - 1)
                .copied()
                .unwrap_or(false)
        })
    }

    fn jvm_in_package(
        &self,
        packages: &HashMap<String, Vec<PathBuf>>,
        path: &[String],
    ) -> Option<JvmType> {
        let (files, names) = (1..path.len()).rev().find_map(|n| {
            let files = packages.get(&path[..n].join("."))?;
            Some((files, &path[n..]))
        })?;
        let mut hits: Vec<Hit> = self
            .jvm_declared(files, names)
            .into_iter()
            .map(|c| c.hit)
            .collect();
        hits.retain(|h| {
            search::jvm_type_name(&h.text).as_deref() == names.last().map(String::as_str)
                && self.jvm_code(h)
        });
        Some(one_type(hits))
    }

    fn jvm_nested(&self, decl: &Hit, name: &str) -> JvmType {
        let Some(text) = self.text_of(&decl.path) else {
            return JvmType::Unknown;
        };
        let lines: Vec<&str> = text.lines().collect();
        let nested: Vec<usize> = search::jvm_members_of(&text, decl.line1, name)
            .into_iter()
            .filter(|&l| search::jvm_type_name(lines[l - 1]).as_deref() == Some(name))
            .collect();
        match nested[..] {
            [line] => JvmType::Project(Hit {
                deleted: None,
                path: decl.path.clone(),
                line1: line,
                byte_col: None,
                text: lines[line - 1].to_owned(),
            }),
            _ => JvmType::Unknown,
        }
    }

    pub(super) fn jvm_lombok(
        &mut self,
        here: &Path,
        text: &str,
        chain: &[String],
        word: &str,
    ) -> Vec<Candidate> {
        let Some(names) = search::jvm_accessor(word).map(|a| a.fields_it_may_read) else {
            return Vec::new();
        };
        if let [owner] = chain
            && owner.starts_with(|c: char| c.is_ascii_uppercase())
        {
            match self.jvm_type_at(here, text, owner) {
                JvmType::Project(decl) => {
                    return self
                        .jvm_hierarchy_member(&decl, word, 0, &mut Vec::new())
                        .into_iter()
                        .map(|hit| Candidate {
                            hit,
                            reason: Reason::Path(owner.clone()),
                        })
                        .collect();
                }
                JvmType::Outside(_) => return Vec::new(),
                JvmType::Unknown => {}
            }
        }
        let mut found = Vec::new();
        for name in &names {
            let pattern = search::def_patterns(Kind::Jvm, name).join("|");
            for h in self.project_definitions(Kind::Jvm, here, name, &pattern) {
                if h.path.extension().is_none_or(|e| e != "java") {
                    continue;
                }
                let Some(t) = self.text_of(&h.path) else {
                    continue;
                };
                let Some(&decl) = search::jvm_enclosing_types(&t, h.line1).first() else {
                    continue;
                };
                if search::jvm_lombok_fields(&t, decl, word).contains(&h.line1) {
                    found.push(Candidate {
                        hit: h,
                        reason: Reason::ByName,
                    });
                }
            }
        }
        self.offer_only |= !found.is_empty();
        found
    }
}

pub(super) fn java_shaped<T>(
    items: Vec<T>,
    hit: impl Fn(&T) -> &Hit,
    word: &str,
    call: bool,
) -> Vec<T> {
    let java = |t: &T| hit(t).path.extension().is_some_and(|e| e == "java");
    if !items.iter().all(java) {
        return items;
    }
    let shaped = |t: &T| search::java_declares_method(&hit(t).text, word) == call;
    if !items.iter().any(shaped) {
        return items;
    }
    items.into_iter().filter(shaped).collect()
}

fn one_type(mut hits: Vec<Hit>) -> JvmType {
    match hits.len() {
        1 => JvmType::Project(hits.remove(0)),
        _ => JvmType::Unknown,
    }
}
