use super::{
    App, Asked, Kind, Near, Path, Patterns, Regex, bound, cs_typed, resolution, rust, search,
};

pub(super) struct Walk {
    pub(super) broke: Option<String>,
    pub(super) cs_walked: Option<Vec<String>>,
    pub(super) declares_here: bool,
}

impl App {
    pub(super) fn answered_by_rust_or_name(
        &mut self,
        a: &Asked,
        near: &Near,
        p: &Patterns,
    ) -> bool {
        let (kind, dotted, chain, word) = (a.kind, a.dotted, &a.chain, &a.word);
        let mut rust_broke = None;
        if kind == Kind::Rust
            && dotted
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
            && chain
                .first()
                .is_some_and(|f| bound(&near.imports, f).is_none())
            && !chain.iter().any(|n| rust::tuple_field(n))
        {
            let after = a.tail.as_str();
            let call = after.starts_with('(') || after.starts_with("::<");
            match self.rust_typed(&a.here, &a.text, word, chain, call) {
                Ok(found) if !found.is_empty() => return self.show_found(a, found),
                Ok(_) => {}
                Err(at) => rust_broke = (chain.len() > 1).then_some(at),
            }
        }
        if kind == Kind::Rust
            && dotted
            && chain.is_empty()
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
            && let Some(receiver) = a
                .before
                .strip_suffix('.')
                .filter(|r| r.ends_with([')', '?']))
        {
            let after = a.tail.as_str();
            let call = after.starts_with('(') || after.starts_with("::<");
            if let Some(found) = self.rust_call_chain(&a.here, &a.text, receiver, word, call)
                && !found.is_empty()
            {
                return self.show_found(a, found);
            }
        }
        if kind == Kind::Rust
            && let Some(found) = self.rust_early(&a.here, &a.text, word, a.range.clone(), dotted)
        {
            self.show_definitions(kind, word, &a.here, found, rust_broke.as_deref());
            return true;
        }
        if kind == Kind::Rust
            && p.on_value
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
        {
            let found = self.rust_methods(&a.here, word, &a.text, chain);
            self.show_definitions(kind, word, &a.here, found, rust_broke.as_deref());
            return true;
        }
        let js_private_member = word.starts_with('#');
        if !dotted && !js_private_member {
            let found = self.implementations(kind, &a.here, &a.text, word);
            if !found.is_empty() {
                return self.show_found(a, found);
            }
        }
        if !dotted && search::field_decl_at(kind, &a.text, self.line + 1, a.range.start, word) {
            let found = self.field_namesakes(kind, &a.here, word);
            return self.show_found(a, found);
        }
        false
    }

    pub(super) fn declares_here(&self, a: &Asked) -> bool {
        if a.kind == Kind::Swift && search::swift_right_of_binding(&a.lead) {
            return false;
        }
        let mut declaring = search::def_patterns(a.kind, &a.word);
        declaring.extend(search::member_or_signature(a.kind, &a.word).unwrap_or_default());
        Regex::new(&declaring.join("|")).is_ok_and(|re| re.is_match(self.line_str()))
    }

    pub(super) fn answered_by_go_ts_swift(
        &mut self,
        a: &Asked,
        near: &Near,
        p: &Patterns,
        declares_here: bool,
    ) -> bool {
        let (kind, dotted, chain, word, locals) =
            (a.kind, a.dotted, &a.chain, &a.word, &near.locals);
        if kind == Kind::Go
            && !dotted
            && !a.go_keyed
            && locals.is_empty()
            && !declares_here
            && bound(&near.imports, word).is_none()
        {
            let found = self.go_bare(&a.here, word, &near.imports);
            return self.show_found(a, found);
        }
        if kind == Kind::TsJs
            && dotted
            && let [name] = chain.as_slice()
            && let [binding] = locals.as_slice()
            && let Some(line) = search::ts_literal_key(&self.buf.lines, *binding, name, word)
        {
            let found = self.lines_here(a, [line], &super::Reason::Local);
            return self.show_found(a, found);
        }
        if kind == Kind::Swift
            && !dotted
            && chain.is_empty()
            && (locals.is_empty() && search::swift_no_local(&a.text, self.line + 1, word))
            && !declares_here
            && !matches!(word.as_str(), "self" | "super" | "init")
            && let Some(mut found) = self.swift_self_members(&a.here, &a.text, word, &p.pattern)
        {
            self.swift_by_labels(word, a.range.end, &mut found);
            return self.show_found(a, found);
        }
        if kind == Kind::Swift
            && dotted
            && chain.is_empty()
            && !matches!(word.as_str(), "init" | "none" | "some")
            && let Some(lead) = a.lead.strip_suffix('.')
            && let Some(found) = self.swift_expected_rows(&a.here, &a.text, lead, word)
        {
            if found.is_empty() {
                self.message = resolution(word, None, &[], None, false);
            } else {
                self.show_found(a, found);
            }
            return true;
        }
        if kind == Kind::Swift
            && dotted
            && !chain.is_empty()
            && chain[0] != "super"
            && word != "init"
            && bound(&near.imports, &chain[0]).is_none()
            && let Some(mut found) = self.swift_typed(&a.here, &a.text, word, chain)
        {
            self.swift_by_labels(word, a.range.end, &mut found);
            if found.is_empty() {
                self.message = resolution(word, None, &[], None, false);
            } else {
                self.show_found(a, found);
            }
            return true;
        }
        false
    }

    pub(super) fn answered_by_cs(
        &mut self,
        a: &Asked,
        near: &Near,
        p: &Patterns,
        walk: &mut Walk,
    ) -> bool {
        let (kind, dotted, chain, word) = (a.kind, a.dotted, &a.chain, &a.word);
        if kind != Kind::CSharp {
            return false;
        }
        let answer = match dotted {
            false => self.cs_initializer(&a.here, word, &a.range).map(Ok),
            true if !chain.is_empty() && bound(&near.imports, &chain[0]).is_none() => {
                Some(self.cs_typed(&a.here, word, chain))
            }
            true if chain.is_empty() => self
                .cs_constructed(&a.here, word, &a.before)
                .or_else(|| self.cs_call_result(&a.here, word, a.range.start))
                .map(Ok),
            true => None,
        };
        match answer {
            Some(Ok(cs_typed::CsAnswer::Found(found))) => return self.show_found(a, found),
            Some(Ok(cs_typed::CsAnswer::Outside { links: via })) => {
                self.offer_only = false;
                self.truncated.set(false);
                self.message = format!("no definition for {word} in the project (via {via})");
                return true;
            }
            Some(Err(at)) => walk.broke = (chain.len() > 1).then_some(at),
            None => {}
        }
        if !dotted && chain.is_empty() && near.locals.is_empty() && !a.before.ends_with("::") {
            match self.cs_class_first(&a.here, word, p.cs_type) {
                Ok(found) => return self.show_found(a, found),
                Err(walked) => walk.cs_walked = walked.filter(|_| !walk.declares_here),
            }
        }
        false
    }

    pub(super) fn answered_by_typed(
        &mut self,
        a: &Asked,
        near: &Near,
        p: &Patterns,
        walk: &mut Walk,
    ) -> bool {
        let (kind, dotted, chain, word) = (a.kind, a.dotted, &a.chain, &a.word);
        let head = (dotted && chain.is_empty())
            .then(|| search::call_head(kind, &a.written, a.start))
            .flatten();
        if kind == Kind::Go
            && head.is_none()
            && dotted
            && chain.is_empty()
            && let Some(answer) = self.go_call_chain(&a.here, &a.written, a.start, word)
        {
            match answer {
                Ok(found) if !found.is_empty() => return self.show_found(a, found),
                Ok(_) => {}
                Err(via) => {
                    self.offer_only = false;
                    self.truncated.set(false);
                    self.message = format!("no definition for {word} in the project (via {via})");
                    return true;
                }
            }
        }
        if !(dotted
            && matches!(kind, Kind::Python | Kind::TsJs | Kind::Go)
            && (head.is_some()
                || chain
                    .first()
                    .is_some_and(|f| bound(&near.imports, f).is_none())))
        {
            return false;
        }
        match self.typed_definitions(kind, &a.here, word, chain, head.as_ref()) {
            Ok(found)
                if a.own()
                    && matches!(found.as_slice(), [c] if c.hit.line1 == self.line + 1 && c.hit.path == a.here) =>
            {
                let found = self.field_namesakes(kind, &a.here, word);
                return self.show_found(a, found);
            }
            Ok(found) if !found.is_empty() => return self.show_found(a, found),
            Ok(_) => {
                if let Some(ty) = self.inherited_outside(kind, &a.here, chain, head.as_ref()) {
                    let found = self.subclass_members(kind, &a.here, word, &p.pattern, &ty);
                    return self.show_found(a, found);
                }
            }
            Err(at) => {
                if let Some(via) = self.builtin_receiver(kind, &a.here, chain, head.as_ref()) {
                    self.offer_only = false;
                    self.truncated.set(false);
                    self.message = format!("{word}: builtin, no source (via {via})");
                    return true;
                }
                if head.is_none()
                    && kind == Kind::TsJs
                    && let Some(via) = self.missing_package_receiver(&a.here, chain, &at)
                {
                    self.offer_only = false;
                    self.truncated.set(false);
                    self.message = format!("no definition for {word} in the project (via {via})");
                    return true;
                }
                let names = head
                    .as_ref()
                    .map_or(chain.len(), |h| h.fields_to_word.len() + 1);
                walk.broke = (names > 1).then_some(at);
            }
        }
        false
    }

    fn missing_package_receiver(&self, here: &Path, chain: &[String], at: &str) -> Option<String> {
        let kind = Kind::TsJs;
        let upto = chain.iter().position(|l| l == at)?;
        let (file, text, bindings, _) = self.last_bindings(kind, here, &chain[..=upto])?;
        let imports = search::imports(kind, &text);
        let dir = file.parent().unwrap_or_else(|| Path::new(""));
        let mut via = None;
        for b in &bindings {
            let search::Value::Type(written) = &b.value else {
                continue;
            };
            let parts = search::type_path(kind, written)?;
            let path = bound(&imports, &parts[0])?;
            if !search::package_missing(&self.root, &self.files, dir, &path) {
                return None;
            }
            let parts_of_name = if path[0].starts_with('@') { 2 } else { 1 };
            let package = path[..parts_of_name.min(path.len())].join("/");
            via.get_or_insert_with(|| format!("{at}: {}, {package} not installed", written.trim()));
        }
        via
    }

    pub(super) fn answered_by_rust_crate(&mut self, a: &Asked, near: &Near) -> bool {
        if a.kind == Kind::Rust
            && !a.dotted
            && near.locals.is_empty()
            && let Some(found) = self.rust_crate_path(
                &a.here,
                &a.text,
                &a.word,
                &a.chain,
                rust::MacroUse::after_word(a.tail.as_str()),
            )
        {
            return self.show_found(a, found);
        }
        false
    }
}
