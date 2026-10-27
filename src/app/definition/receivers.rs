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
        let mut std = None;
        if kind == Kind::Rust
            && dotted
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
            && chain
                .first()
                .is_some_and(|f| bound(&near.imports, f).is_none() && !rust::tuple_field(f))
        {
            let tupled = chain.iter().any(|n| rust::tuple_field(n));
            let after = a.tail.as_str();
            let call = after.starts_with('(') || after.starts_with("::<");
            match self.rust_typed(&a.here, &a.text, word, chain, call) {
                Ok(rust::Typing::Members(found) | rust::Typing::Std(found, _))
                    if !found.is_empty() =>
                {
                    return self.show_found(a, found);
                }
                Ok(rust::Typing::Std(_, t)) => std = Some(t),
                Ok(rust::Typing::Members(_)) => {}
                Err(at) => rust_broke = (chain.len() > 1 && !tupled).then_some(at),
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
                .filter(|r| r.ends_with([')', '?']) || r.contains(").") || r.contains("?."))
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
            && dotted
            && chain.is_empty()
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
            && a.tail.starts_with('(')
            && let Some(found) = self.rust_array_call(&a.text, &a.before, word)
        {
            if !found.is_empty() {
                return self.show_found(a, found);
            }
            std = Some("[T; N]".to_owned());
        }
        if kind == Kind::Rust
            && dotted
            && chain.is_empty()
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
        {
            let call = a.tail.starts_with('(') || a.tail.starts_with("::<");
            match self.rust_index_member(&a.here, &a.text, &a.before, word, call) {
                Some(rust::Typing::Members(found) | rust::Typing::Std(found, _))
                    if !found.is_empty() =>
                {
                    return self.show_found(a, found);
                }
                Some(rust::Typing::Std(_, t)) => std = Some(t),
                _ => {}
            }
        }
        if kind == Kind::Rust
            && let Some(found) = self.rust_early(&a.here, &a.text, word, a.range.clone(), dotted)
        {
            self.show_definitions(kind, word, &a.here, found, rust_broke.as_deref());
            return true;
        }
        if kind == Kind::Rust
            && (p.on_value || std.is_some())
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && word != "await"
        {
            let found = self.rust_methods(&a.here, word, &a.text, chain, &a.before, std.as_deref());
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
        if self.answered_by_swift_cast(a) {
            return true;
        }
        if kind == Kind::Swift
            && dotted
            && chain.is_empty()
            && word != "init"
            && let Some(head) = search::call_head(kind, &a.written, a.start)
            && head.fields_to_word.is_empty()
            && let Some(mut found) =
                self.swift_on_call(&a.here, &a.text, word, &head.call_without_arguments, &[])
        {
            self.swift_by_labels(word, a.range.end, &mut found);
            if !found.is_empty() {
                return self.show_found(a, found);
            }
        }
        if kind == Kind::Swift
            && dotted
            && chain.is_empty()
            && word != "init"
            && let Some(links) = a.written.get(..a.start).and_then(search::swift_call_links)
            && let Some(mut found) = self.swift_on_call(
                &a.here,
                &a.text,
                word,
                &format!("{}()", links[0]),
                &links[1..],
            )
        {
            self.swift_by_labels(word, a.range.end, &mut found);
            if !found.is_empty() {
                return self.show_found(a, found);
            }
        }
        self.answered_by_swift_outside_call(a)
    }

    fn answered_by_swift_outside_call(&mut self, a: &Asked) -> bool {
        let answered = a.kind == Kind::Swift
            && a.dotted
            && a.chain.is_empty()
            && a.written
                .get(..a.start)
                .is_some_and(|before| self.swift_outside_call(&a.here, &a.text, before, &a.word));
        if answered {
            self.message = resolution(&a.word, None, &[], None, false);
        }
        answered
    }

    fn answered_by_swift_cast(&mut self, a: &Asked) -> bool {
        if a.kind != Kind::Swift || !a.dotted || !a.chain.is_empty() {
            return false;
        }
        let Some(found) = search::swift_cast_receiver(&a.lead)
            .and_then(|cast| self.swift_cast_rows(&a.here, &cast, &a.word))
        else {
            return false;
        };
        if found.is_empty() {
            self.message = resolution(&a.word, None, &[], None, false);
        } else {
            self.show_found(a, found);
        }
        true
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
                Err(walked) => {
                    if walked.is_some()
                        && let Some(found) = self.cs_aliased(&a.here, word, &a.range)
                    {
                        return self.show_found(a, found);
                    }
                    walk.cs_walked = walked.filter(|_| !walk.declares_here);
                }
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
        if self.store_read(a, near)
            && let [name] = chain.as_slice()
            && let Some(found) = self.store_member(&a.here, &near.imports, name, word)
        {
            return self.show_found(a, found);
        }
        let head = (dotted && chain.is_empty())
            .then(|| search::call_head(kind, &a.written, a.start))
            .flatten();
        if matches!(kind, Kind::Go | Kind::Python)
            && head.is_none()
            && dotted
            && chain.is_empty()
            && let Some(answer) = self.call_chain(kind, &a.here, &a.written, a.start, word)
        {
            match answer {
                Ok(found) if !found.is_empty() => return self.show_found(a, found),
                Ok(_) => {}
                Err(via) => {
                    return self.none_found(format!(
                        "no definition for {word} in the project (via {via})"
                    ));
                }
            }
        }
        if kind == Kind::Python
            && dotted
            && let Some(literal) = search::python_str_literal_receiver(&a.before)
        {
            return self.none_found(format!("{word}: builtin, no source (via {literal})"));
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
                let set_from_outside = chain.last().is_some_and(|r| {
                    Regex::new(&format!(
                        r"\b{}\.{}\s*=[^=]",
                        regex::escape(r),
                        regex::escape(word)
                    ))
                    .is_ok_and(|re| re.is_match(&a.text))
                });
                if !set_from_outside
                    && let Some(via) = self.closed_class(kind, &a.here, chain, head.as_ref(), word)
                {
                    return self.none_found(format!(
                        "no definition for {word} in the project (via {via})"
                    ));
                }
            }
            Err(at) => {
                if let Some(via) = self.builtin_receiver(kind, &a.here, chain, head.as_ref()) {
                    return self.none_found(format!("{word}: builtin, no source (via {via})"));
                }
                if kind == Kind::Go
                    && let Some(via) = self.go_outside_receiver(&a.here, chain, head.as_ref())
                {
                    return self.none_found(format!(
                        "no definition for {word} in the project (via {via})"
                    ));
                }
                if let Some(via) = self.js_std_receiver(a) {
                    return self.none_found(format!("{word}: builtin, no source (via {via})"));
                }
                if head.is_none()
                    && kind == Kind::TsJs
                    && let Some(via) = self.missing_package_receiver(&a.here, chain, &at)
                {
                    return self.none_found(format!(
                        "no definition for {word} in the project (via {via})"
                    ));
                }
                let names = head
                    .as_ref()
                    .map_or(chain.len(), |h| h.fields_to_word.len() + 1);
                walk.broke = (names > 1).then_some(at);
            }
        }
        false
    }

    fn go_outside_receiver(
        &self,
        here: &Path,
        chain: &[String],
        head: Option<&search::CallHead>,
    ) -> Option<String> {
        if let Some(head) = head {
            let search::Value::Call(callee) = &head.value else {
                return None;
            };
            let text = self.buf.lines.join("\n");
            return (head.fields_to_word.is_empty()
                && self.go_outside_call(here, &text, self.line + 1, callee))
            .then(|| format!("{callee}()"));
        }
        let last = chain.last()?;
        let (file, text, bindings, mut links) = self.last_bindings(Kind::Go, here, chain)?;
        let mut written = None;
        for b in &bindings {
            let this = match &b.value {
                search::Value::Type(t) | search::Value::New(t)
                    if self.go_outside(&file, &text, t) =>
                {
                    t.trim_start_matches('*').to_owned()
                }
                search::Value::Call(callee)
                    if self.go_outside_call(&file, &text, b.line1, callee) =>
                {
                    format!("{callee}()")
                }
                _ => return None,
            };
            written.get_or_insert(this);
        }
        links.push(format!("{last}: {}", written?));
        Some(links.join(" \u{2192} "))
    }

    fn go_outside_call(&self, file: &Path, text: &str, line1: usize, callee: &str) -> bool {
        let Some((package, _)) = callee.split_once('.') else {
            return false;
        };
        let args = text
            .lines()
            .nth(line1.saturating_sub(1))
            .and_then(|line| search::go_call_arguments(line, callee));
        args.is_some_and(|args| {
            search::bindings(Kind::Go, text, line1, package).is_empty()
                && self.go_outside(file, text, callee)
                && args
                    .iter()
                    .all(|arg| self.go_outside_value(file, text, line1, arg))
        })
    }

    fn go_outside_value(&self, file: &Path, text: &str, line1: usize, arg: &str) -> bool {
        const PREDECLARED: &str = "any bool byte complex64 complex128 error float32 float64 int \
            int8 int16 int32 int64 rune string uint uint8 uint16 uint32 uint64 uintptr";
        let number = arg.strip_prefix('-').unwrap_or(arg);
        if arg.starts_with(['"', '`', '\''])
            || number.starts_with(|c: char| c.is_ascii_digit())
            || matches!(arg, "true" | "false" | "nil")
        {
            return true;
        }
        let bindings = search::bindings(Kind::Go, text, line1, arg);
        let [
            search::Binding {
                value: search::Value::Type(t),
                ..
            },
        ] = bindings.as_slice()
        else {
            return false;
        };
        let base = t.trim_start_matches(['[', ']', '*', '.']);
        PREDECLARED.split(' ').any(|p| p == base) || self.go_outside(file, text, base)
    }

    fn none_found(&mut self, message: String) -> bool {
        self.offer_only = false;
        self.truncated.set(false);
        self.message = message;
        true
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
