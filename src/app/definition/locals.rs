use super::{App, Asked, Kind, Reason, import_line, names_itself, search, whole_at};

pub(super) struct Near {
    pub(super) imports: Vec<search::Import>,
    pub(super) locals: Vec<usize>,
    pub(super) swift_binds_here: bool,
    pub(super) arrow_param: bool,
    pub(super) unbound: bool,
}

impl App {
    pub(super) fn scope_near(&mut self, a: &Asked) -> Option<Near> {
        let (kind, word, range) = (a.kind, &a.word, &a.range);
        let in_literal = search::literal_lines(kind, &a.text).get(self.line) == Some(&true);
        let mut imports = if in_literal {
            search::imports_as_written(kind, &a.text)
        } else {
            search::imports(kind, &a.text)
        };
        let line = self.line_str();
        let swift_binds_here = kind == Kind::Swift
            && !a.dotted
            && line.get(range.clone()) == Some(word.as_str())
            && search::swift_binds_on(line, word)
            && !search::swift_binds_on(&format!("{}_{}", a.lead, a.tail), word);
        let declared = swift_binds_here
            || (a.chain.is_empty() && search::binds_at(kind, self.line_str(), range.start, word));
        let mut locals = self.locals_of(a, declared);
        let go_own = kind == Kind::Go
            && locals.contains(&(self.line + 1))
            && search::go_binds_here(self.line_str(), word, range.start);
        if go_own {
            locals = vec![self.line + 1];
        }
        let own_arrow = kind == Kind::TsJs
            && !a.dotted
            && locals.contains(&(self.line + 1))
            && search::ts_arrow_binds(self.line_str(), word, range.start);
        let arrow_param = kind == Kind::TsJs
            && !a.dotted
            && search::ts_arrow_param(self.line_str(), word, range.start);
        let swift_closure = kind == Kind::Swift
            && !a.dotted
            && search::swift_closure_param(self.line_str(), word, range.start);
        let ruby_block = kind == Kind::Ruby
            && !a.dotted
            && search::ruby_block_param(self.line_str(), word, range.start);
        if own_arrow || arrow_param || swift_closure || ruby_block {
            locals = vec![self.line + 1];
        }
        let type_param = (kind == Kind::TsJs
            && !a.dotted
            && a.chain.is_empty()
            && locals.is_empty()
            && imports.iter().all(|i| i.name != *word))
        .then(|| search::ts_type_param(&a.text, self.line + 1, word))
        .flatten();
        if let Some(p) = &type_param {
            locals = vec![p.line1];
        }
        if !locals.is_empty() {
            imports.retain(|i| i.name != a.first());
        }
        let star = kind == Kind::Python && imports.iter().any(|i| i.name == "*");
        let unbound = kind == Kind::Python
            && !a.dotted
            && a.chain.is_empty()
            && search::bindings(kind, &a.text, self.line + 1, word).is_empty()
            && !search::python_class_binds(&a.text, self.line + 1, word)
            && !names_itself(kind, self.line_str(), word)
            && !self.offer_only;
        if unbound && !star && search::PYTHON_BUILTINS.contains(&word.as_str()) {
            self.offer_only = false;
            self.message = format!("{word}: builtin, no source");
            return None;
        }
        let same_line = (matches!(kind, Kind::Jvm | Kind::PowerShell | Kind::Dart)
            || (kind == Kind::Swift && !declared))
            && locals == [self.line + 1]
            && whole_at(self.line_str(), word, "").is_some_and(|at| at < range.start);
        let own_line = go_own
            || ruby_block
            || (kind == Kind::Ruby
                && !a.dotted
                && locals == [self.line + 1]
                && search::ruby_assigns_before(self.line_str(), word, range.start))
            || (kind == Kind::CSharp
                && locals == [self.line + 1]
                && search::cs_binds_here(self.line_str(), word, range.start));
        let python_inline = kind == Kind::Python
            && !declared
            && locals == [self.line + 1]
            && search::python_inline_binds(self.line_str(), word);
        if self.answered_by_locals(
            a,
            &locals,
            same_line || own_line || own_arrow || python_inline || type_param.is_some(),
        ) {
            return None;
        }
        Some(Near {
            imports,
            locals,
            swift_binds_here,
            arrow_param,
            unbound,
        })
    }

    fn locals_of(&self, a: &Asked, declared: bool) -> Vec<usize> {
        let (kind, word, range, chain) = (a.kind, &a.word, &a.range, &a.chain);
        let (before, dotted, first) = (a.before.as_str(), a.dotted, a.first());
        let key = kind == Kind::Lua
            && (search::table_key(&a.text, self.line + 1, range)
                || (before.ends_with(':') && !before.ends_with("::")));
        let after = a.tail.as_str();
        let key = key
            || a.elixir_call()
            || (kind == Kind::Ruby && after.starts_with('('))
            || (a.here.extension().is_some_and(|e| e == "java")
                && !dotted
                && after.trim_start().starts_with('(')
                && !before.trim_end().ends_with("new"))
            || search::erlang(&a.here)
            || (kind == Kind::Elixir
                && !dotted
                && (before.ends_with(':') || (after.starts_with(':') && !after.starts_with("::"))))
            || (kind == Kind::Dart
                && after.starts_with(':')
                && search::dart_label(&a.text, self.line + 1, range.start));
        let super_keyword = !dotted && word == "super" && kind != Kind::Lua;
        let after = a.tail.trim_start();
        let rust_path = kind == Kind::Rust
            && (before.ends_with("::")
                || after.starts_with("::")
                || (after.starts_with('!') && !after.starts_with("!=")));
        let rust_field =
            kind == Kind::Rust && after.starts_with(':') && !after.starts_with("::") && !a.captured;
        let bare = kind == Kind::TsJs && !dotted && chain.is_empty();
        let ts_key = bare
            && a.lead.trim().is_empty()
            && (after.starts_with(':') || after.starts_with("?:"))
            && !after.starts_with("::");
        let required = match kind {
            Kind::TsJs => search::ts_import_lines1(&a.text, first),
            _ => Vec::new(),
        };
        let closure = |n: usize| {
            (kind == Kind::Swift || kind == Kind::Python && search::python_in_function(&a.text, n))
                && !dotted
                && chain.is_empty()
                && n != self.line + 1
        };
        let locals_at = |text: &str, line: usize| -> Vec<usize> {
            let binding: Vec<usize> = if declared {
                vec![self.line + 1]
            } else {
                self.binding_lines(kind, text, line, range.start, chain.is_empty(), first)
            };
            binding
                .into_iter()
                .filter(|&n| {
                    let l = &self.buf.lines[n - 1];
                    !key && !super_keyword
                        && (closure(n) || !names_itself(kind, l, first) || bare)
                        && !import_line(kind, l)
                        && !rust_path
                        && (!rust_field || n == self.line + 1)
                        && (!ts_key || n == self.line + 1)
                })
                .map(|n| match kind {
                    Kind::TsJs => search::written_line(&self.buf.lines, n, first),
                    Kind::CSharp => search::cs_written_line(&self.buf.lines, n, first),
                    _ => n,
                })
                .filter(|n| !required.contains(n) && !self.template_shadows(&a.here, first))
                .collect()
        };
        let mut locals = locals_at(&a.text, self.line + 1);
        if kind == Kind::Shell
            && locals.is_empty()
            && search::shell_command_position(&a.lead) == Some(false)
        {
            let lines: Vec<&str> = a.text.lines().collect();
            let literal = search::literal_lines(kind, &a.text);
            locals.extend(search::shell_assigned(&lines, &literal, self.line, first));
        }
        if let [only] = locals[..]
            && bare
            && only != self.line + 1
        {
            locals.extend(search::ts_var_again(&a.text, only, first));
        }
        if bare
            && !ts_key
            && locals.is_empty()
            && required.is_empty()
            && let Some(hoisted) = search::ts_hoisted_function(&a.text, self.line + 1, first)
        {
            return vec![hoisted];
        }
        if (bare || self.script_scope(&a.here, Some(first)).is_some()) && locals.is_empty() {
            let module = locals_at(&format!("{}\n0", a.text), self.buf.lines.len() + 1);
            if module.is_empty() && bare && a.text.contains("namespace") {
                return search::ts_merged_namespace(&a.text, self.line + 1, first);
            }
            if !module.contains(&(self.line + 1)) {
                return module;
            }
        }
        locals
    }

    fn answered_by_locals(&mut self, a: &Asked, locals: &[usize], ours: bool) -> bool {
        let kind = a.kind;
        let on_itself = match kind {
            Kind::C => {
                search::c_bindings_at(&a.text, self.line + 1, Some(a.range.start), a.first())
                    .iter()
                    .all(|p| p.line1 == self.line + 1 && p.byte_col == a.range.start)
            }
            _ => *locals == [self.line + 1],
        };
        if !a.dotted
            && !a.before.ends_with("::")
            && !locals.is_empty()
            && (!on_itself
                || ours
                || matches!(
                    kind,
                    Kind::Rust
                        | Kind::Nix
                        | Kind::Haskell
                        | Kind::Julia
                        | Kind::Gdscript
                        | Kind::Solidity
                )
                || search::lisp(kind))
        {
            let found = self.lines_here(a, locals.iter().copied(), &Reason::Local);
            return self.show_found(a, found);
        }
        false
    }
}
