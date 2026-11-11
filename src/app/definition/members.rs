use super::{App, Asked, Candidate, Kind, Near, Reason, bound, is_word, names_itself, search};
use crate::app::jvm_typed::java_shaped;
use std::path::Path;

pub(super) struct Patterns {
    pub(super) pattern: String,
    pub(super) members: Option<String>,
    pub(super) php_members: Option<String>,
    pub(super) on_value: bool,
    pub(super) cs_type: bool,
    pub(super) ruby_local: bool,
}

impl App {
    pub(super) fn answered_by_members(&mut self, a: &Asked, near: &Near) -> bool {
        self.answered_by_class_member(a, near) || self.answered_by_qualifier(a, near)
    }

    fn answered_by_class_member(&mut self, a: &Asked, near: &Near) -> bool {
        let (kind, dotted, chain, locals) = (a.kind, a.dotted, &a.chain, &near.locals);
        let before = a.before.as_str();
        if kind == Kind::C
            && !dotted
            && chain.is_empty()
            && locals.is_empty()
            && !before.ends_with("::")
            && let Some(class) = search::c_method_class(&a.text, self.line + 1, &a.word)
        {
            let found = self.c_class_fields(&a.here, &a.word, &class, false);
            if !found.is_empty() {
                return self.show_found(a, found);
            }
            if let Some(found) = self.cpp_class_scope(&a.here, &a.text, &a.word, &class) {
                return self.show_found(a, found);
            }
        }
        let constructed = before
            .trim_end()
            .strip_suffix("new")
            .is_some_and(|b| !b.ends_with(is_word));
        let members = (kind == Kind::Jvm
            && chain.is_empty()
            && !dotted
            && !before.ends_with("::")
            && locals.is_empty())
        .then(|| self.jvm_members(&a.here, &a.text, before, &a.word))
        .flatten();
        let nested_type =
            |c: &Candidate| search::jvm_type_name(&c.hit.text).is_some_and(|name| name == a.word);
        let nested = constructed && members.as_ref().is_some_and(|f| f.iter().all(nested_type));
        if (nested || !constructed)
            && let Some(mut found) = members
        {
            if nested {
                found.extend(self.jvm_constructors(&found, &a.word));
            }
            let found = self.jvm_fit_candidates(&a.here, &a.word, a.range.clone(), chain, found);
            return self.show_found(a, found);
        }
        if kind == Kind::Jvm
            && !nested
            && locals.is_empty()
            && !(dotted && chain.is_empty())
            && !names_itself(kind, self.line_str(), &a.word)
            && let Some(found) = self.jvm_imported(&a.text, chain, &a.word, a.range.clone(), false)
        {
            return self.show_found(a, found);
        }
        false
    }

    fn answered_by_qualifier(&mut self, a: &Asked, near: &Near) -> bool {
        let (kind, dotted, chain) = (a.kind, a.dotted, &a.chain);
        let before = a.before.as_str();
        if kind == Kind::Lua {
            let chain = match before.strip_suffix(':') {
                Some(b) if !b.ends_with(':') => search::qualifier(&format!("{b}."), a.start),
                _ => chain.clone(),
            };
            let rebound = chain.first().is_some_and(|f| {
                search::lua_binds_before(&a.lead, f, true)
                    || search::lua_reassigned(&a.text, self.line + 1, f)
            });
            let found = if rebound {
                Vec::new()
            } else {
                self.lua_qualified(&a.here, &a.text, &chain, before, &a.word)
            };
            let on = |c: &Candidate| c.hit.path == a.here && c.hit.line1 == self.line + 1;
            if !found.iter().all(on) {
                return self.show_found(a, found);
            }
        }
        if kind == Kind::Go
            && !dotted
            && a.tail.starts_with('.')
            && let Some(path) = bound(&near.imports, &a.word)
            && let Some(line) = search::go_import_line1(&a.text, &a.word)
            && !search::go_may_declare(&a.text, self.line + 1, &a.word)
            && self
                .package_declarations(kind, &a.here, &a.word, None)
                .is_empty()
        {
            let hit = self.line_hit(&a.here, line);
            let reason = Reason::Import(path.join("/"));
            return self.show_found(a, vec![Candidate { hit, reason }]);
        }
        if kind == Kind::Jvm
            && (dotted || before.ends_with("::"))
            && (!a.own()
                || search::jvm_enclosing_types(&a.text, self.line + 1).is_empty()
                || (a.here.extension().is_some_and(|e| e == "kt" || e == "kts")
                    && search::kotlin_lambda_receiver(&a.text, self.line + 1, before).is_some()))
        {
            let literal = dotted
                && chain.is_empty()
                && before.strip_suffix('.').is_some_and(|b| b.ends_with('"'));
            let found = if literal {
                self.jvm_outside(&a.here, &a.word, "String").map(|hits| {
                    hits.into_iter()
                        .map(|hit| Candidate {
                            hit,
                            reason: Reason::Receiver("String".into()),
                        })
                        .collect()
                })
            } else {
                self.jvm_typed(&a.here, &a.text, before, chain, &a.word)
            };
            if let Some(found) = found {
                let call = a.tail.trim_start().starts_with('(') || before.ends_with("::");
                let found = match a.here.extension() {
                    Some(e) if e == "java" => java_shaped(found, |c| &c.hit, &a.word, call),
                    _ => found,
                };
                let found =
                    self.jvm_fit_candidates(&a.here, &a.word, a.range.clone(), chain, found);
                return self.show_found(a, found);
            }
        }
        false
    }

    fn ruby_answered_on_value(&mut self, a: &Asked) -> bool {
        let word = &a.word;
        if word == "super" && !a.dotted && a.chain.is_empty() {
            let found = self.ruby_super(&a.here, &a.text);
            return !found.is_empty() && self.show_found(a, found);
        }
        if !a.dotted || a.own() {
            return false;
        }
        let found = self.ruby_receiver_methods(&a.here, &a.text, &a.before, word);
        if !found.is_empty() {
            return self.show_found(a, found);
        }
        self.truncated.set(false);
        let Some(via) = self.ruby_core_receiver(&a.here, &a.text, &a.before, word) else {
            return false;
        };
        self.offer_only = false;
        self.message = format!("{word}: builtin, no source (via {via})");
        true
    }

    pub(super) fn store_read(&self, a: &Asked, near: &Near) -> bool {
        let chain = &a.chain;
        a.kind == Kind::TsJs
            && a.here.extension().is_some_and(|e| e == "svelte")
            && matches!(&chain[..], [head] if a.before.ends_with(&format!("${head}.")))
            && bound(&near.imports, &chain[0]).is_none_or(|path| {
                path.split_last().is_some_and(|(_, module)| {
                    !search::module_files(a.kind, &self.root, &self.files, &a.here, module)
                        .is_empty()
                })
            })
    }

    pub(super) fn patterns(&mut self, a: &Asked, near: &Near) -> Option<Patterns> {
        let (kind, dotted, chain, word) = (a.kind, a.dotted, &a.chain, &a.word);
        let before = a.before.as_str();
        let own = a.own();
        let referenced = kind == Kind::Jvm
            && before.ends_with("::")
            && chain.first().is_some_and(|f| f != "super");
        let php_members = (kind == Kind::Php && dotted).then(|| {
            let call = a.tail.trim_start().starts_with('(');
            search::php_member_patterns(word, call).join("|")
        });
        let store = self.store_read(a, near);
        let on_value = (dotted || referenced)
            && !own
            && (kind == Kind::Php
                || store
                || chain
                    .first()
                    .is_none_or(|f| bound(&near.imports, f).is_none()));
        let members = match &php_members {
            Some(m) => on_value.then(|| m.clone()),
            None => on_value
                .then(|| search::member_patterns(kind, word))
                .flatten()
                .map(|m| m.join("|")),
        };
        let mut patterns = search::def_patterns(kind, word);
        let cs_type = kind == Kind::CSharp
            && !dotted
            && chain.is_empty()
            && self.cs_types_only(&a.here, word, a.range.clone());
        if cs_type {
            patterns = search::cs_type_patterns(word);
        }
        let lower =
            kind == Kind::Ruby && word.starts_with(|c: char| c.is_ascii_lowercase() || c == '_');
        if kind == Kind::Ruby && (dotted || lower) {
            search::cut_def_patterns(kind, &mut patterns, &[search::ruby_assignment(word)]);
        }
        let ruby_local = lower && !a.tail.starts_with('(');
        if lower
            && near.locals.is_empty()
            && ((!dotted && chain.is_empty() && !before.ends_with("::")) || own)
            && (!ruby_local || search::ruby_locals(&a.text, self.line + 1, word).is_empty())
            && !search::ruby_self_rebound(&a.text, self.line + 1, before)
        {
            let found = self.ruby_self_methods(&a.here, &a.text, word);
            if !found.is_empty() {
                self.show_found(a, found);
                return None;
            }
            self.truncated.set(false);
        }
        if lower
            && chain.last().is_none_or(|l| l == "new" || l == "instance")
            && let Some(made) = search::ruby_constructed_receiver(before)
        {
            let found = self.ruby_constructed(&a.here, &made, word);
            if !found.is_empty() {
                self.show_found(a, found);
                return None;
            }
            self.truncated.set(false);
        }
        if lower && self.ruby_answered_on_value(a) {
            return None;
        }
        if kind == Kind::Elixir && dotted {
            patterns.retain(|p| !p.starts_with(r"^\s*@"));
        }
        self.narrow(kind, &mut patterns, &a.text, a.range.clone());
        if patterns.is_empty() {
            self.message = self.no_rules();
            return None;
        }
        let pattern = patterns.join("|");
        if kind == Kind::Php
            && let Some(found) =
                self.php_early(&a.here, &a.text, before, chain, word, a.range.clone())
        {
            self.show_found(a, found);
            return None;
        }
        Some(Patterns {
            pattern,
            members,
            php_members,
            on_value,
            cs_type,
            ruby_local,
        })
    }
}

impl App {
    pub(super) fn statics_through_type(
        &self,
        kind: Kind,
        word: &str,
        here: &Path,
        found: &mut Vec<Candidate>,
    ) {
        if !matches!(kind, Kind::Jvm | Kind::CSharp | Kind::TsJs) || found.len() < 2 {
            return;
        }
        let line = self.line_str();
        let Some((r, w)) = search::definition_word(Some(kind), line, self.col) else {
            return;
        };
        let text = self.buf.lines.join("\n");
        let qualifier = line
            .get(..r.start)
            .and_then(|b| b.strip_suffix('.'))
            .and_then(|b| {
                let start = b
                    .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                    .map_or(0, |i| i + 1);
                let (head, name) = (b.get(..start)?, b.get(start..)?);
                let packaged = || {
                    let head = head.strip_suffix('.')?;
                    let from = head
                        .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.'))
                        .map_or(0, |i| i + 1);
                    let mut chain: Vec<String> =
                        head.get(from..)?.split('.').map(str::to_owned).collect();
                    chain.push(name.to_owned());
                    (kind == Kind::Jvm)
                        .then(|| self.jvm_packaged_type(here, &text, &chain))
                        .flatten()
                };
                (!head.ends_with('.') || packaged().is_some()).then_some(name)
            });
        let Some(qualifier) = qualifier.filter(|q| w == word && q.starts_with(char::is_uppercase))
        else {
            return;
        };
        let line1 = self.line + 1;
        if !search::path_bindings(kind, here, &text, line1, qualifier).is_empty()
            || (kind == Kind::Jvm
                && search::jvm_enclosing_types(&text, line1)
                    .into_iter()
                    .any(|d| !search::jvm_members_of(here, &text, d, qualifier).is_empty()))
        {
            return;
        }
        let owned: Option<Vec<bool>> = found
            .iter()
            .map(|c| {
                let text = self.text_of(&c.hit.path)?;
                let (stat, name) = static_owner(kind, &text, c.hit.line1, word)?;
                (name == qualifier).then_some(stat)
            })
            .collect();
        let Some(owned) = owned else { return };
        if owned.iter().any(|s| *s) && owned.iter().any(|s| !*s) {
            let mut kept = owned.into_iter();
            found.retain(|_| kept.next().unwrap_or(false));
        }
    }
}

fn static_owner(kind: Kind, text: &str, line1: usize, word: &str) -> Option<(bool, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let decl = lines.get(line1.checked_sub(1)?)?;
    let written_static = decl
        .find(word)
        .and_then(|at| decl.get(..at))
        .is_some_and(|head| head.split_whitespace().any(|t| t == "static"));
    match kind {
        Kind::Jvm => {
            let owners = search::jvm_enclosing_types(text, line1);
            let first = lines.get(owners.first()?.checked_sub(1)?)?;
            if first.contains("companion object") {
                let outer = lines.get(owners.get(1)?.checked_sub(1)?)?;
                return Some((true, search::jvm_type_name(outer)?));
            }
            let object = first.split_whitespace().any(|t| t == "object");
            Some((written_static || object, search::jvm_type_name(first)?))
        }
        Kind::CSharp => {
            let owner = search::cs_owner(text, line1)?;
            Some((
                written_static,
                search::cs_type_decl(lines.get(owner - 1)?)?.name,
            ))
        }
        _ => {
            let owner = search::owner_decl(kind, text, line1)?;
            Some((
                written_static,
                search::type_name(kind, lines.get(owner - 1)?)?,
            ))
        }
    }
}
