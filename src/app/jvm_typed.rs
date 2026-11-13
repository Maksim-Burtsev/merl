use super::typed::Base;
use super::{App, Candidate, HashMap, Hit, Kind, Path, PathBuf, Reason, Regex, search};

mod head;
mod lambda;
mod scala_unpacked;
mod top_level;

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
            return self.jvm_typed_member(here, ty, &cast, word, true);
        }
        let made = search::jvm_constructed_receiver(before, kotlin).or_else(|| {
            search::scala(here)
                .then(|| search::scala_packaged_construction(before))
                .flatten()
        });
        if let ([], Some(made)) = (chain, made) {
            return match self.jvm_made(here, text, &made, kotlin) {
                ty @ JvmType::Project(_) => self.jvm_typed_member(here, ty, &made, word, true),
                _ => None,
            };
        }
        if let ([], false, Some(source)) = (chain, kotlin, search::java_element_access(before)) {
            let line = Self::jvm_declared_line(here, text, self.line + 1, &source)?;
            let (container, element) = search::java_element_type(line, &source)?;
            let label = format!("{source}: element {element}");
            return self.jvm_held_member(here, text, &container, &element, &label, word);
        }
        let kt = here.extension().is_some_and(|e| e == "kt" || e == "kts");
        if let ([], true, Some(source)) = (chain, kt, search::kotlin_map_read(before))
            && let Some((container, value)) =
                Self::jvm_declared_line(here, text, self.line + 1, &source)
                    .and_then(|line| search::kotlin_map_value(line, &source))
        {
            let label = format!("{source}: value {value}");
            return self.jvm_held_member(here, text, &container, &value, &label, word);
        }
        let safe: Vec<String>;
        let chain = match chain {
            [] if kotlin => {
                let mut plain = before.replace("?.", ".").replace("!!.", ".");
                if kt && let Some(receiver) = search::kotlin_scope_returned(&plain) {
                    plain = receiver;
                }
                safe = search::qualifier(&plain, plain.len());
                if let ([], true, Some(c)) = (&safe[..], kt, search::kotlin_call_chain(&plain)) {
                    let (ty, links, instance) =
                        self.kotlin_chain_type(here, text, self.line + 1, Some(before), &c)?;
                    return self.jvm_typed_member(
                        here,
                        ty,
                        &links.join(" \u{2192} "),
                        word,
                        instance,
                    );
                }
                if safe.is_empty() {
                    return self.scala_static_chain(here, text, &plain, word);
                }
                &safe[..]
            }
            [] => {
                safe = match search::jvm_getter_chain(before) {
                    c if c.is_empty() => search::java_fluent_chain(before)
                        .map(|(head, calls)| {
                            let calls = calls.into_iter().map(|c| format!("{c}()"));
                            std::iter::once(head).chain(calls).collect()
                        })
                        .or_else(|| self.java_bare_chain(here, text, before))
                        .unwrap_or_default(),
                    c => c,
                };
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
            return self.jvm_typed_member(here, ty, &format!("{named}.super"), word, false);
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
        let (ty, links, rest, instance) = self.jvm_head(here, text, before, first, rest, outer)?;
        let (ty, links) = self.jvm_follow(ty, links, first, rest, instance)?;
        let instance = instance || !rest.is_empty();
        self.jvm_typed_member(here, ty, &links.join(" \u{2192} "), word, instance)
    }

    fn scala_static_chain(
        &self,
        here: &Path,
        text: &str,
        plain: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let c = search::kotlin_call_chain(plain)
            .filter(|c| search::scala(here) && !c.constructed && !c.called)?;
        let (ty, links, rest) = self.jvm_static_head(here, text, &c.head, &c.links)?;
        let (ty, links) = self.jvm_follow(ty, links, &c.head, rest, false)?;
        self.jvm_typed_member(here, ty, &links.join(" \u{2192} "), word, false)
    }

    fn java_bare_chain(&self, here: &Path, text: &str, before: &str) -> Option<Vec<String>> {
        let calls = search::java_bare_chain(before)?;
        let line = self.line + 1;
        let innermost = *search::jvm_enclosing_types(text, line).first()?;
        search::jvm_type_name(text.lines().nth(innermost.checked_sub(1)?)?)?;
        (!search::java_new_between(text, line, before, innermost)).then_some(())?;
        let first = self.jvm_returned(here, text, line, None, calls.first()?, false)?;
        matches!(first.resolved, JvmType::Project(_)).then_some(())?;
        let calls = calls.into_iter().map(|c| format!("{c}()"));
        Some(std::iter::once("this".to_owned()).chain(calls).collect())
    }

    fn jvm_static_head<'r>(
        &self,
        here: &Path,
        text: &str,
        first: &str,
        rest: &'r [String],
    ) -> Option<(JvmType, Vec<String>, &'r [String])> {
        let upper = |n: &str| n.starts_with(|c: char| c.is_ascii_uppercase());
        let (mut decl, mut links, mut rest) = if upper(first) {
            let decl = match self.jvm_type_at(here, text, first) {
                JvmType::Project(decl) => decl,
                JvmType::Unknown => self.scala_companion(here, first, true)?,
                JvmType::Outside(_) => return None,
            };
            (decl, vec![first.to_owned()], rest)
        } else {
            let n = rest.iter().position(|l| upper(l))?;
            let path: Vec<String> = std::iter::once(first)
                .chain(rest.get(..=n)?.iter().map(String::as_str))
                .map(str::to_owned)
                .collect();
            (!path.iter().any(|l| l.ends_with("()"))).then_some(())?;
            let decl = self.jvm_packaged_type(here, text, &path)?;
            (decl, vec![path.join(".")], rest.get(n + 1..)?)
        };
        while let Some((link, more)) = rest.split_first()
            && upper(link)
            && !link.ends_with("()")
            && let JvmType::Project(inner) = self.jvm_nested(&decl, link)
        {
            decl = inner;
            links.push(link.clone());
            rest = more;
        }
        let next = rest.first()?;
        if !next.ends_with("()") {
            search::path_bindings(Kind::Jvm, here, text, self.line + 1, first)
                .is_empty()
                .then_some(())?;
            let Some(field) = self.jvm_field_type(&decl, next, false) else {
                let [_] = self.jvm_enum_constant(&decl, next)[..] else {
                    return None;
                };
                links.push(next.clone());
                return Some((JvmType::Project(decl), links, rest.get(1..)?));
            };
            (field.hit.text.split_whitespace().any(|t| t == "static")
                || self.jvm_object_member(&decl, &field.hit))
            .then_some(())?;
        }
        Some((JvmType::Project(decl), links, rest))
    }

    fn jvm_object_member(&self, decl: &Hit, member: &Hit) -> bool {
        decl.text.split_whitespace().any(|w| w == "object")
            || (member.path == decl.path
                && self
                    .text_of(&member.path)
                    .is_some_and(|t| companion_held(&t, member.line1, decl.line1)))
    }

    fn scala_companion(&self, file: &Path, written: &str, object: bool) -> Option<Hit> {
        if !search::scala(file) || written.contains('.') {
            return None;
        }
        let pattern = search::def_patterns(Kind::Jvm, written).join("|");
        let cut = self.truncated.get();
        let mut hits = self.project_definitions(Kind::Jvm, file, written, &pattern);
        let full = self.truncated.get() && !cut;
        self.truncated.set(cut);
        hits.retain(|h| self.jvm_code(h));
        let is_object = |h: &Hit| h.text.split_whitespace().any(|w| w == "object");
        let [a, b] = hits.as_slice() else {
            return None;
        };
        let named = |h: &Hit| search::jvm_type_name(&h.text).as_deref() == Some(written);
        (!full && a.path == b.path && named(a) && named(b) && is_object(a) != is_object(b))
            .then_some(())?;
        Some(if is_object(a) == object { a } else { b }.clone())
    }

    fn jvm_follow(
        &self,
        mut ty: JvmType,
        mut links: Vec<String>,
        first: &str,
        rest: &[String],
        instance: bool,
    ) -> Option<(JvmType, Vec<String>)> {
        for (i, field) in rest.iter().enumerate() {
            let JvmType::Project(decl) = ty else {
                return None;
            };
            let instance = instance || i > 0;
            let TypedField { hit, written } = match field.strip_suffix("()") {
                Some(call) => self.jvm_call_type(&decl, call, instance)?,
                None => self.jvm_field_type(&decl, field, instance)?,
            };
            let t = self.text_of(&hit.path)?;
            ty = match self.jvm_type_on(&hit.path, &t, &written, Some(hit.line1)) {
                JvmType::Unknown => self
                    .scala_companion(&hit.path, &written, false)
                    .map_or(JvmType::Unknown, JvmType::Project),
                ty => ty,
            };
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
        line1: usize,
        before: Option<&str>,
        c: &search::KotlinCallChain,
    ) -> Option<(JvmType, Vec<String>, bool)> {
        if c.links.len() > 6 {
            return None;
        }
        let (ty, links, rest, instance) = if c.constructed {
            (
                self.jvm_made(here, text, &c.head, true),
                vec![format!("{}()", c.head)],
                &c.links[..],
                true,
            )
        } else if c.called {
            let ahead = before.or_else(|| text.lines().nth(line1.checked_sub(1)?))?;
            let w = self
                .kotlin_top_level_returned(here, text, line1, ahead, &c.head)
                .or_else(|| self.kotlin_member_returned(here, text, line1, ahead, &c.head))?;
            let link = format!("{}(): {}", c.head, w.written);
            (w.resolved, vec![link], &c.links[..], true)
        } else if let Some((ty, links, rest)) = self.jvm_static_head(here, text, &c.head, &c.links)
        {
            (ty, links, rest, false)
        } else {
            let (ty, links) = self.jvm_chain_head(here, text, before?, &c.head, None)?;
            (ty, links, &c.links[..], true)
        };
        let (ty, links) = self.jvm_follow(ty, links, &c.head, rest, instance)?;
        Some((ty, links, instance || !rest.is_empty()))
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
            if let Some(call) = d.source.strip_suffix("()") {
                return self.kotlin_top_level_component(file, text, line1, call, d.index);
            }
            let source = if d.in_loop {
                let element = Self::jvm_element(file, text, line1, &d.source)?;
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
            && search::path_bindings(Kind::Jvm, file, text, line1.checked_sub(1)?, name).is_empty()
        {
            let members = search::jvm_enclosing_types(text, line1)
                .into_iter()
                .map(|d| search::jvm_members_of(file, text, d, name))
                .find(|m| !m.is_empty())?;
            let [m] = members[..] else {
                return None;
            };
            let written = search::jvm_declared_type(text.lines().nth(m - 1)?, name, true)?;
            let resolved = self.jvm_type_at(file, text, &written);
            return Some(WrittenType { resolved, written });
        }
        let chain = search::kotlin_call_chain(&format!("{init}."))?;
        if !chain.constructed
            && chain.head.starts_with(|c: char| c.is_ascii_uppercase())
            && !init.trim_start().starts_with(&format!("{}.", chain.head))
        {
            return None;
        }
        let (resolved, links, _) = self.kotlin_chain_type(file, text, line1, None, &chain)?;
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
            f if f.starts_with(|c: char| c.is_ascii_alphabetic()) => {
                if search::jvm_rebound_before(before, f)
                    && !search::path_bindings(Kind::Jvm, here, text, line, f)
                        .iter()
                        .any(|b| b.line1 == line)
                {
                    return None;
                }
                if search::scala(here) {
                    let bound: Vec<usize> = search::jvm_bindings_at(here, text, line, self.col, f)
                        .iter()
                        .map(|b| b.line1)
                        .collect();
                    if search::scala_rebound(here, text, line, before, f, &bound) {
                        return None;
                    }
                }
                let smart = kotlin.then(|| search::jvm_smart_cast(text, line, before, f));
                let callee = || {
                    kt.then(|| search::kotlin_lambda_callee(here, text, line, before, f))
                        .flatten()
                };
                let narrowed = smart.flatten().or_else(callee).or_else(|| {
                    let lambda =
                        kt.then(|| search::kotlin_lambda_collection(here, text, line, before, f));
                    let lambda = lambda.flatten()?;
                    match lambda.strip_suffix(".entries") {
                        Some(e) => match self.jvm_type_at(here, text, e) {
                            JvmType::Project(h) if h.text.contains("enum class") => {
                                Some(e.to_owned())
                            }
                            _ => None,
                        },
                        None => Self::jvm_element(here, text, line, &lambda),
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
        let found: Vec<Candidate> =
            (self.jvm_hierarchy_member(&decl, word, 0, &mut Vec::new(), false))
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect();
        (!found.is_empty()).then_some(found)
    }

    fn jvm_made(&self, here: &Path, text: &str, written: &str, kotlin: bool) -> JvmType {
        let renamed = || {
            search::jvm_imports_at(here, text, self.line + 1)
                .iter()
                .any(|i| i.bound_name == written && i.path.last().is_some_and(|l| l != written))
        };
        if !written.contains('.') && !renamed() {
            let found = self
                .jvm_members(here, text, "", written)
                .or_else(|| self.jvm_imported_all(text, &[], written, false))
                .unwrap_or_default();
            let named =
                |c: &Candidate| search::jvm_type_name(&c.hit.text).as_deref() == Some(written);
            let alias = |c: &Candidate| {
                search::kotlin_typealias(&c.hit.text).is_some_and(|(name, _)| name == written)
            };
            if kotlin && !found.iter().all(named) && !found.iter().all(alias) {
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
            let JvmType::Project(b) = self.jvm_type_on(here, text, &base, Some(decl)) else {
                return None;
            };
            let reason = Reason::Receiver(format!("super: {base}"));
            found.extend(
                self.jvm_hierarchy_member(&b, word, 0, &mut Vec::new(), false)
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

    pub(super) fn jvm_packaged_type(
        &self,
        here: &Path,
        text: &str,
        chain: &[String],
    ) -> Option<Hit> {
        let (owner, package) = chain.split_last()?;
        let first = package.first()?;
        if !owner.starts_with(|c: char| c.is_ascii_uppercase())
            || !package
                .iter()
                .all(|p| p.starts_with(|c: char| c.is_ascii_lowercase()))
            || self.jvm_value_named(here, text, first)
        {
            return None;
        }
        let packages = self.jvm_packages()?;
        match self.jvm_in_package(&packages, chain)? {
            JvmType::Project(decl) => Some(decl),
            _ => None,
        }
    }

    pub(super) fn jvm_enum_case(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
        found: &mut Vec<Candidate>,
    ) {
        let java = here.extension().is_some_and(|e| e == "java");
        if java && search::java_case_label(before) {
            if let Some(selector) = search::java_switch_selector(text, self.line + 1)
                && let Some((JvmType::Project(decl), _)) =
                    self.jvm_chain_head(here, text, "", &selector, None)
            {
                let cases = self.jvm_enum_constant(&decl, word);
                if !cases.is_empty() {
                    *found = cases;
                }
            }
            return;
        }
        if search::scala(here) {
            self.scala_enum_case(here, text, word, found);
            return;
        }
        let own = search::jvm_enclosing_types(text, self.line + 1)
            .into_iter()
            .any(|d| {
                search::enum_constants(text, d)
                    .iter()
                    .any(|c| c.name == word)
            });
        if !found.is_empty() || own {
            return;
        }
        let imports = search::jvm_enum_imports(here, text, word);
        let single = imports.iter().any(|i| i.single);
        for import in imports.iter().filter(|i| i.single == single) {
            if let JvmType::Project(decl) = self.jvm_type_at(here, text, &import.owner) {
                found.extend(self.jvm_enum_constant(&decl, word));
            }
        }
    }

    pub(super) fn jvm_enum_constant(&self, decl: &Hit, word: &str) -> Vec<Candidate> {
        let Some(name) = search::jvm_type_name(&decl.text) else {
            return Vec::new();
        };
        let is_enum = Regex::new(&format!(
            r"\benum\s+(?:class\s+)?{}\b",
            regex::escape(&name)
        ))
        .is_ok_and(|re| re.is_match(&decl.text));
        let Some(text) = self.text_of(&decl.path).filter(|_| is_enum) else {
            return Vec::new();
        };
        let lines: Vec<&str> = text.lines().collect();
        search::enum_constants(&text, decl.line1)
            .into_iter()
            .filter(|c| c.name == word)
            .map(|c| Candidate {
                hit: Hit {
                    deleted: None,
                    path: decl.path.clone(),
                    line1: c.line1,
                    byte_col: None,
                    text: lines
                        .get(c.line1 - 1)
                        .copied()
                        .unwrap_or_default()
                        .to_owned(),
                },
                reason: Reason::Path(name.clone()),
            })
            .collect()
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
        self.jvm_typed_member(here, JvmType::Project(decl), owner, word, false)
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
        instance: bool,
    ) -> Option<Vec<Candidate>> {
        let found = match ty {
            JvmType::Unknown => return None,
            JvmType::Outside(name) => self.jvm_outside(here, word, &name)?,
            JvmType::Project(decl) => {
                let mut names = Vec::new();
                let mut hits = self.jvm_hierarchy_member(&decl, word, 0, &mut names, instance);
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
        let elsewhere = off.iter().any(|h| {
            search::scala(&h.path) == search::scala(here)
                && self.jvm_extension_receiver(h, word).is_some()
        });
        (!elsewhere || !on.is_empty()).then_some(on)
    }

    fn jvm_extensions(&self, here: &Path, word: &str) -> Vec<Hit> {
        let pattern = search::def_patterns(Kind::Jvm, word).join("|");
        let cut = self.truncated.get();
        let mut hits = self.project_definitions(Kind::Jvm, here, word, &pattern);
        self.truncated.set(cut);
        hits.retain(|h| self.jvm_receiver_of(h, word).is_some());
        hits
    }

    pub(super) fn jvm_receiver_of(&self, hit: &Hit, word: &str) -> Option<String> {
        self.jvm_extension_receiver(hit, word)
            .or_else(|| search::scala_implicit_receiver(&self.text_of(&hit.path)?, hit.line1, word))
    }

    fn jvm_extension_receiver(&self, hit: &Hit, word: &str) -> Option<String> {
        search::jvm_receiver(&hit.text, word)
            .or_else(|| search::jvm_receiver_at(&self.text_of(&hit.path)?, hit.line1, word))
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
            r"\b{0}\s+instanceof\s+[\w$.]+(?:<[^>]*>)?\s*(?:[^\w\s]|$)"
        };
        let narrowed = Regex::new(&narrowing.replace("{0}", &regex::escape(name))).ok()?;
        let own_when = kotlin
            .then(|| search::jvm_when_else(text, line1, name))
            .flatten();
        let lines: Vec<&str> = text.lines().collect();
        let ext = file.extension().and_then(|e| e.to_str());
        let start = search::jvm_function_start(&lines, line1);
        let elsewhere = |at: usize| {
            start.is_some_and(|s| {
                at > line1 || at < s || search::kotlin_check_closed(text, line1, at, name)
            })
        };
        let mentions: Vec<Option<usize>> = narrowed
            .find_iter(text)
            .map(|m| text.get(..m.start()).map(|t| t.matches('\n').count() + 1))
            .filter(|&at| at != own_when)
            .collect();
        if !mentions.iter().all(|at| at.is_some_and(elsewhere)) {
            return None;
        }
        let literal_typed = matches!(ext, Some("java" | "kt" | "kts"));
        let cursor_line = line1 == self.line + 1 && lines.get(self.line) == Some(&self.line_str());
        let bindings: Vec<usize> = if cursor_line {
            search::jvm_bindings_at(file, text, line1, self.col, name)
        } else {
            search::path_bindings(Kind::Jvm, file, text, line1, name)
        }
        .iter()
        .map(|b| b.line1)
        .collect();
        let declared = if bindings.is_empty() {
            match search::jvm_enclosing_members(file, text, line1, name) {
                Some(m) => m,
                None if matches!(ext, Some("kt" | "kts" | "scala")) => {
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
            let declared = search::jvm_declared_type(at, name, kotlin).or_else(|| {
                (ext == Some("scala"))
                    .then(|| search::scala_generic_declared_type(at, name))
                    .flatten()
                    .filter(|t| {
                        matches!(
                            self.jvm_type_on(file, text, t, Some(l)),
                            JvmType::Project(_)
                        )
                    })
            });
            let this = match (declared, literal) {
                (None, Some(t)) => WrittenType {
                    resolved: JvmType::Outside(t.to_owned()),
                    written: t.to_owned(),
                },
                (Some(t), _) => WrittenType {
                    resolved: self.jvm_type_on(file, text, &t, Some(l)),
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
                    None if ext == Some("java") => self
                        .java_static_initialized(file, text, l, name)
                        .or_else(|| self.java_lambda_element(file, text, l, name))?,
                    None if ext == Some("scala") => self
                        .scala_tuple_returned(file, text, l, at, name)
                        .or_else(|| self.scala_extracted(file, text, l, at, name))?,
                    None => return None,
                },
                (None, None) => return None,
            };
            if found.as_ref().is_some_and(|f| f.written != this.written) {
                return None;
            }
            found = Some(this);
        }
        found.filter(|f| mentions.is_empty() || matches!(f.resolved, JvmType::Project(_)))
    }

    fn jvm_held_member(
        &self,
        here: &Path,
        text: &str,
        container: &str,
        held: &str,
        label: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        if let JvmType::Project(hit) = self.jvm_type_at(here, text, container)
            && self.jvm_sees(here, text, container, &hit)
        {
            return None;
        }
        let ty = self.jvm_type_at(here, text, held);
        self.jvm_typed_member(here, ty, label, word, true)
    }

    fn jvm_element(file: &Path, text: &str, line1: usize, name: &str) -> Option<String> {
        let line = Self::jvm_declared_line(file, text, line1, name)?;
        search::kotlin_element_type(line, name)
    }

    fn jvm_declared_line<'a>(
        file: &Path,
        text: &'a str,
        line1: usize,
        name: &str,
    ) -> Option<&'a str> {
        let mut declared: Vec<usize> = search::path_bindings(Kind::Jvm, file, text, line1, name)
            .iter()
            .map(|b| b.line1)
            .collect();
        if declared.is_empty() {
            declared = search::jvm_enclosing_types(text, line1)
                .into_iter()
                .map(|d| search::jvm_members_of(file, text, d, name))
                .find(|m| !m.is_empty())?;
        }
        let [l] = declared[..] else {
            return None;
        };
        text.lines().nth(l.checked_sub(1)?)
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
        if recv.is_none() && search::jvm_enclosing_types(text, line1).is_empty() {
            let at = text.lines().nth(line1.checked_sub(1)?)?;
            return self.kotlin_top_level_returned(file, text, line1, at, call);
        }
        let decl = self.jvm_call_owner(file, text, line1, recv, kotlin)?;
        let TypedField { hit: m, written } = self.jvm_call_type(&decl, call, recv.is_some())?;
        (!search::scala(&m.path)).then_some(())?;
        let t = self.text_of(&m.path)?;
        Some(WrittenType {
            resolved: self.jvm_type_at(&m.path, &t, &written),
            written,
        })
    }

    fn jvm_call_owner(
        &self,
        file: &Path,
        text: &str,
        line1: usize,
        recv: Option<&str>,
        kotlin: bool,
    ) -> Option<Hit> {
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
        Some(decl)
    }

    fn jvm_call_type(&self, decl: &Hit, call: &str, instance: bool) -> Option<TypedField> {
        let found = self.jvm_hierarchy_member(decl, call, 0, &mut Vec::new(), instance);
        let found = java_shaped(found, |h| h, call, true);
        if let [one, more @ ..] = found.as_slice()
            && !more.is_empty()
            && !search::scala(&one.path)
        {
            let mk = one.path.extension().is_none_or(|e| e != "java");
            let returned = |h: &Hit| {
                (h.path == one.path)
                    .then(|| search::jvm_return_type(&h.text, call, mk))
                    .flatten()
            };
            let written = returned(one)?;
            more.iter()
                .all(|h| returned(h).as_ref() == Some(&written))
                .then_some(())?;
            return Some(self.jvm_substituted(decl, one.clone(), written));
        }
        let [hit] = found.try_into().ok()?;
        let mk = hit.path.extension().is_none_or(|e| e != "java");
        let written = search::jvm_return_type(&hit.text, call, mk).or_else(|| {
            let getter = search::jvm_accessor(call).filter(|a| !a.setter)?;
            getter
                .fields_it_may_read
                .iter()
                .find_map(|n| search::jvm_declared_type(&hit.text, n, mk))
        })?;
        Some(self.jvm_substituted(decl, hit, written))
    }

    fn jvm_field_type(&self, decl: &Hit, field: &str, instance: bool) -> Option<TypedField> {
        let mut names = Vec::new();
        let found = self.jvm_hierarchy_member(decl, field, 0, &mut names, instance);
        let [hit] = java_shaped(found, |h| h, field, false).try_into().ok()?;
        let kotlin = hit.path.extension().is_none_or(|e| e != "java");
        let written = search::jvm_declared_type(&hit.text, field, kotlin)?;
        Some(self.jvm_substituted(decl, hit, written))
    }

    fn jvm_substituted(&self, decl: &Hit, hit: Hit, written: String) -> TypedField {
        self.jvm_class_argument(decl, &hit, &written)
            .unwrap_or(TypedField { hit, written })
    }

    fn jvm_class_argument(&self, decl: &Hit, member: &Hit, written: &str) -> Option<TypedField> {
        if search::scala(&member.path) || search::jvm_type_parameter(&member.text, written) {
            return None;
        }
        let text = self.text_of(&member.path)?;
        let holder = search::jvm_enclosing_types(&text, member.line1)
            .into_iter()
            .next()?;
        let name = search::jvm_type_name(text.lines().nth(holder - 1)?)?;
        let params = search::generic_header(Kind::Jvm, &text, holder, &name)?.params;
        let index = params.iter().position(|p| p == written)?;
        let (hit, written) =
            super::typed::base_argument(decl, (index, params.len()), 0, &|d: &Hit| {
                let text = self.text_of(&d.path).filter(|_| !search::scala(&d.path))?;
                let name = search::jvm_type_name(&d.text)?;
                let header = search::generic_header(Kind::Jvm, &text, d.line1, &name)?;
                let mut bases = search::jvm_bases(&text, d.line1, false);
                bases.extend(search::java_implements(&text, d.line1));
                let bases = bases
                    .into_iter()
                    .filter_map(|base| {
                        let JvmType::Project(b) =
                            self.jvm_type_on(&d.path, &text, &base, Some(d.line1))
                        else {
                            return None;
                        };
                        Some(if (&b.path, b.line1) == (&member.path, holder) {
                            Base::Holder(search::base_arguments(&header.rest, &base))
                        } else {
                            Base::Other(b)
                        })
                    })
                    .collect();
                Some((header.params, bases))
            })?;
        Some(TypedField { hit, written })
    }

    pub(super) fn jvm_hierarchy_member(
        &self,
        decl: &Hit,
        word: &str,
        depth: usize,
        walked: &mut Vec<String>,
        instance: bool,
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
        let mut found = search::jvm_members_of(&decl.path, &text, decl.line1, word);
        found.retain(|&l| !instance || !companion_held(&text, l, decl.line1));
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
            match self.jvm_type_on(&decl.path, &text, &base, Some(decl.line1)) {
                JvmType::Project(b) => {
                    out.extend(self.jvm_hierarchy_member(&b, word, depth + 1, walked, instance));
                }
                _ => known = false,
            }
        }
        if out.is_empty() && known {
            for base in search::java_implements(&text, decl.line1) {
                if let JvmType::Project(b) =
                    self.jvm_type_on(&decl.path, &text, &base, Some(decl.line1))
                {
                    out.extend(self.jvm_hierarchy_member(&b, word, depth + 1, walked, instance));
                }
            }
        }
        out
    }

    fn jvm_sees(&self, file: &Path, text: &str, written: &str, decl: &Hit) -> bool {
        if written.contains('.') || decl.path == file {
            return true;
        }
        let package = self
            .text_of(&decl.path)
            .and_then(|t| search::jvm_package(&t));
        package == search::jvm_package(text)
            || search::jvm_imports(file, text).iter().any(|i| {
                i.bound_name == written
                    || (i.bound_name == "*"
                        && i.path.split_last().is_some_and(|(_, base)| {
                            package.as_deref() == Some(base.join(".").as_str())
                        }))
            })
    }

    pub(super) fn jvm_type_at(&self, file: &Path, text: &str, written: &str) -> JvmType {
        self.jvm_type_on(file, text, written, None)
    }

    fn jvm_type_on(&self, file: &Path, text: &str, written: &str, on: Option<usize>) -> JvmType {
        self.jvm_type_hop(file, text, written, on, 8)
    }

    fn jvm_type_hop(
        &self,
        file: &Path,
        text: &str,
        written: &str,
        on: Option<usize>,
        hops: usize,
    ) -> JvmType {
        if let Some(ty) = self.jvm_written_package(file, text, written, on) {
            return ty;
        }
        if let Some((outer, inner)) = written.rsplit_once('.') {
            return match self.jvm_type_hop(file, text, outer, on, hops) {
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
        let imports = on.map_or_else(
            || search::jvm_imports(file, text),
            |line1| search::jvm_imports_at(file, text, line1),
        );
        let bound = |imports: &[search::JvmImport]| {
            imports
                .iter()
                .find(|i| i.bound_name == written)
                .map(|i| i.path.clone())
        };
        if on.is_none()
            && self.rel_current().as_deref() == Some(file)
            && bound(&search::jvm_imports_at(file, text, self.line + 1)) != bound(&imports)
        {
            return JvmType::Unknown;
        }
        if let Some(search::JvmImport { path, .. }) =
            imports.iter().find(|i| i.bound_name == written)
        {
            if self.rel_current().as_deref() == Some(file) {
                let line1 = on.unwrap_or(self.line + 1);
                let nested = Self::jvm_nested_in_scope(file, text, written, line1);
                if !nested.is_empty() {
                    return one_type(nested);
                }
            }
            let Some(packages) = self.jvm_packages() else {
                return JvmType::Unknown;
            };
            let ty = self.jvm_in_package(&packages, path);
            if hops > 0
                && !matches!(ty, Some(JvmType::Project(_)))
                && let Some(aliased) = self.kotlin_imported_alias(file, &packages, path, hops - 1)
            {
                return aliased;
            }
            return ty.unwrap_or_else(|| JvmType::Outside(written.to_owned()));
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
        if hops > 0
            && let Some(ty) = self.kotlin_aliased(file, text, written, &hits, hops - 1)
        {
            return ty;
        }
        let types: Vec<Hit> = hits.iter().filter(|h| is_type(h)).cloned().collect();
        let others = hits.iter().any(|h| {
            !is_type(h)
                && !types.iter().any(|t| t.path == h.path)
                && !self.jvm_hidden_elsewhere(file, h)
        });
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

    pub(super) fn jvm_nested_in_scope(
        file: &Path,
        text: &str,
        written: &str,
        line1: usize,
    ) -> Vec<Hit> {
        let lines: Vec<&str> = text.lines().collect();
        search::jvm_enclosing_types(text, line1)
            .into_iter()
            .map(|decl| {
                search::jvm_members_of(file, text, decl, written)
                    .into_iter()
                    .filter(|&l| search::jvm_type_name(lines[l - 1]).as_deref() == Some(written))
                    .map(|l| Hit {
                        deleted: None,
                        path: file.to_path_buf(),
                        line1: l,
                        byte_col: None,
                        text: lines[l - 1].to_owned(),
                    })
                    .collect::<Vec<Hit>>()
            })
            .find(|found| !found.is_empty())
            .unwrap_or_default()
    }

    fn kotlin_aliased(
        &self,
        file: &Path,
        here: &str,
        written: &str,
        hits: &[Hit],
        hops: usize,
    ) -> Option<JvmType> {
        let aliases: Vec<(&Hit, &str)> = hits
            .iter()
            .filter_map(|h| {
                search::kotlin_typealias(&h.text)
                    .filter(|(name, _)| *name == written)
                    .map(|(_, target)| (h, target))
            })
            .collect();
        let [(hit, target)] = aliases.as_slice() else {
            return None;
        };
        let text = self.text_of(&hit.path)?;
        let package = search::jvm_package(&text);
        let seen = hit.path == file || (package.is_some() && search::jvm_package(here) == package);
        let nested = hits
            .iter()
            .any(|h| !std::ptr::eq(h, *hit) && (h.path == file || h.path == hit.path));
        if !file.extension().is_some_and(|e| e == "kt" || e == "kts") || !seen || nested {
            return None;
        }
        match self.jvm_type_hop(&hit.path, &text, target, Some(hit.line1), hops) {
            JvmType::Project(decl) => Some(JvmType::Project(decl)),
            _ => None,
        }
    }

    fn kotlin_imported_alias(
        &self,
        file: &Path,
        packages: &HashMap<String, Vec<PathBuf>>,
        path: &[String],
        hops: usize,
    ) -> Option<JvmType> {
        if !file.extension().is_some_and(|e| e == "kt" || e == "kts") {
            return None;
        }
        let (name, package) = path.split_last()?;
        let files = packages.get(&package.join("."))?;
        let hits: Vec<Hit> = self
            .jvm_declared(files, std::slice::from_ref(name))
            .into_iter()
            .map(|c| c.hit)
            .filter(|h| self.jvm_code(h))
            .collect();
        let [hit] = hits.as_slice() else {
            return None;
        };
        let (_, target) = search::kotlin_typealias(&hit.text).filter(|(n, _)| n == name)?;
        let text = self.text_of(&hit.path)?;
        match self.jvm_type_hop(&hit.path, &text, target, Some(hit.line1), hops) {
            JvmType::Project(decl) => Some(JvmType::Project(decl)),
            _ => None,
        }
    }

    fn jvm_code(&self, h: &Hit) -> bool {
        self.text_of(&h.path).is_some_and(|t| {
            !search::path_literal_lines(Kind::Jvm, &h.path, &t)
                .get(h.line1 - 1)
                .copied()
                .unwrap_or(false)
        })
    }

    pub(super) fn jvm_in_package(
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
        let nested: Vec<usize> = search::jvm_members_of(&decl.path, &text, decl.line1, name)
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
                        .jvm_hierarchy_member(&decl, word, 0, &mut Vec::new(), false)
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

fn companion_held(text: &str, line1: usize, decl: usize) -> bool {
    static COMPANION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bcompanion\s+object\b").expect("the regex is a literal")
    });
    matches!(search::jvm_enclosing_types(text, line1)[..], [c, d, ..]
        if d == decl && text.lines().nth(c - 1).is_some_and(|l| COMPANION.is_match(l)))
}

fn one_type(mut hits: Vec<Hit>) -> JvmType {
    match hits.len() {
        1 => JvmType::Project(hits.remove(0)),
        _ => JvmType::Unknown,
    }
}
