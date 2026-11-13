use super::{App, Asked, Kind, Reason, Regex, import_line, names_itself, search, whole_at};

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
        let mut imports = self.imports_near(a);
        let swift_binds_here = self.swift_binds_here(a);
        let ts_key = ts_key(a, self.line + 1);
        let declared = swift_binds_here
            || ts_key
            || (a.chain.is_empty() && search::binds_at(kind, self.line_str(), range.start, word));
        let mut locals = self.python_locals(a, self.locals_of(a, declared), declared);
        let go_local_type = a.lead.trim() == "type" && a.lead.starts_with(char::is_whitespace);
        let go_own = kind == Kind::Go
            && (go_local_type
                || locals.contains(&(self.line + 1))
                    && search::go_binds_here(self.line_str(), word, range.start));
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
        let c_parameter = kind == Kind::C
            && !a.dotted
            && search::c_names_a_parameter(&a.text, self.line + 1, range.start, word);
        let ts_own = !arrow_param && self.ts_declares_here(a);
        if own_arrow || arrow_param || swift_closure || ruby_block || c_parameter || ts_own {
            locals = vec![self.line + 1];
        }
        let type_param = (locals.is_empty() || kind == Kind::Python)
            .then(|| self.type_param(a, &imports))
            .flatten()
            .filter(|&p| locals.is_empty() || search::python_locals_outside(&a.text, p, &locals));
        if let Some(line1) = type_param {
            locals = vec![line1];
        }
        if !locals.is_empty() || self.def_hides_import(a) {
            imports.retain(|i| i.name != a.first());
        }
        let star = kind == Kind::Python && imports.iter().any(|i| i.name == "*");
        let unbound = kind == Kind::Python
            && !a.dotted
            && a.chain.is_empty()
            && search::path_bindings(kind, &a.here, &a.text, self.line + 1, word).is_empty()
            && !search::python_class_binds(&a.text, self.line + 1, word)
            && !names_itself(kind, self.line_str(), word)
            && !self.offer_only;
        if unbound && !star && search::PYTHON_BUILTINS.contains(&word.as_str()) {
            self.offer_only = false;
            self.message = format!("{word}: builtin, no source");
            return None;
        }
        let same_line = (matches!(kind, Kind::Jvm | Kind::PowerShell | Kind::Dart)
            || (matches!(kind, Kind::Swift | Kind::Lua) && !declared))
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
        let python_own = self.python_own(a, &mut locals, declared);
        if self.answered_by_locals(
            a,
            &locals,
            same_line
                || own_line
                || own_arrow
                || ts_own
                || ts_key
                || python_own
                || c_parameter
                || type_param.is_some(),
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

    fn type_param(&self, a: &Asked, imports: &[search::Import]) -> Option<usize> {
        let (kind, word, line1) = (a.kind, &a.word, self.line + 1);
        if a.dotted
            || !a.chain.is_empty()
            || (kind != Kind::TsJs
                && search::in_prose(kind, &a.here, &a.text, line1, a.range.start))
        {
            return None;
        }
        match kind {
            Kind::TsJs => search::ts_type_param(&a.text, line1, word)
                .filter(|p| {
                    p.typed
                        || imports.iter().all(|i| i.name != *word)
                        || ts_type_position(a, self.line_str())
                })
                .map(|p| p.line1),
            Kind::Python if search::python_in_default(&a.text, line1, a.range.start) => None,
            Kind::Jvm | Kind::Dart | Kind::Python => {
                search::generic_param_line(kind, &a.here, &a.text, line1, word)
            }
            _ if a.before.ends_with("::") => None,
            Kind::CSharp => search::cs_generic_param_line(&a.text, line1, word),
            Kind::Rust if !rust_type_position(a) => None,
            Kind::Rust | Kind::Go | Kind::C => search::type_param_line(kind, &a.text, line1, word),
            _ => None,
        }
    }

    fn imports_near(&self, a: &Asked) -> Vec<search::Import> {
        let kind = a.kind;
        if search::path_literal_lines(kind, &a.here, &a.text).get(self.line) == Some(&true) {
            return search::imports_as_written(kind, &a.text);
        }
        let mut imports = search::imports(kind, &a.text);
        if kind == Kind::Go && a.chain.len() == 1 && imports.iter().all(|i| i.name != a.first()) {
            imports.extend(self.go_import_by_package(a));
        }
        imports
    }

    fn go_import_by_package(&self, a: &Asked) -> Option<search::Import> {
        let name = a.first();
        search::go_unaliased_imports(&a.text)
            .into_iter()
            .find(|path| {
                search::module_files(Kind::Go, &self.root, &self.files, &a.here, path)
                    .first()
                    .and_then(|f| self.text_of(f))
                    .is_some_and(|t| {
                        t.lines()
                            .find_map(|l| l.strip_prefix("package "))
                            .and_then(|p| p.split_whitespace().next())
                            == Some(name)
                    })
            })
            .map(|path| search::Import {
                name: name.to_owned(),
                path,
            })
    }

    fn ts_declares_here(&self, a: &Asked) -> bool {
        let word = &a.word;
        if a.kind != Kind::TsJs
            || a.dotted
            || !a.chain.is_empty()
            || whole_at(self.line_str(), word, "") != Some(a.range.start)
            || whole_at(&a.tail, word, "").is_some()
            || !search::ts_import_lines1(&a.text, word).is_empty()
        {
            return false;
        }
        let Ok(bare) = Regex::new(&format!(
            r"(?:^|[^\w$.]){}(?:[^\w$]|$)",
            regex::escape(word)
        )) else {
            return false;
        };
        let literal = search::path_literal_lines(a.kind, &a.here, &a.text);
        let Some(used) = (self.line + 1..self.buf.lines.len()).find(|&k| {
            let l = self.buf.lines[k].trim_start();
            !literal.get(k).copied().unwrap_or(true)
                && !l.starts_with("//")
                && !l.starts_with('*')
                && bare.is_match(l)
        }) else {
            return false;
        };
        self.binding_lines(a.kind, &a.text, used + 1, 0, true, word)
            .into_iter()
            .map(|n| search::written_line(&self.buf.lines, n, word))
            .any(|n| n == self.line + 1)
    }

    fn swift_binds_here(&self, a: &Asked) -> bool {
        let (word, range, line) = (&a.word, &a.range, self.line_str());
        let wrapped = a.kind == Kind::Swift && search::swift_wrapped_params(&a.text, self.line + 1);
        let signature = |l: String| if wrapped { format!("func f({l})") } else { l };
        a.kind == Kind::Swift
            && !a.dotted
            && line.get(range.clone()) == Some(word.as_str())
            && search::swift_binds_on(&signature(line.to_owned()), word)
            && !search::swift_binds_on(&signature(format!("{}_{}", a.lead, a.tail)), word)
    }

    fn def_hides_import(&self, a: &Asked) -> bool {
        a.kind == Kind::Python && search::python_def_hides_import(&a.text, self.line + 1, a.first())
    }

    fn python_locals(&self, a: &Asked, mut locals: Vec<usize>, declared: bool) -> Vec<usize> {
        let (line, word) = (self.line_str(), &a.word);
        let elsewhere = matches!(a.kind, Kind::Python | Kind::Starlark)
            && !declared
            && !a.dotted
            && locals.contains(&(self.line + 1))
            && search::python_binds_only_inline(line, word)
            && !search::python_binds_around(line, word, a.range.start);
        if elsewhere {
            locals.retain(|&n| n != self.line + 1);
        }
        locals
    }

    fn python_own(&self, a: &Asked, locals: &mut Vec<usize>, declared: bool) -> bool {
        let (line, own) = (self.line_str(), self.line + 1);
        let around = matches!(a.kind, Kind::Python | Kind::Starlark)
            && !declared
            && !a.dotted
            && locals.contains(&own)
            && (search::python_binds_around(line, &a.word, a.range.start)
                || search::python_def_line_binds(line, &a.word, a.range.start));
        if around {
            *locals = vec![own];
        }
        around
            || (a.kind == Kind::Python
                && *locals == [own]
                && !a.dotted
                && search::python_in_function_body(&a.text, own))
    }

    fn declares_itself(&self, kind: Kind, n: usize, name: &str) -> bool {
        names_itself(kind, &self.buf.lines[n - 1], name)
            || kind == Kind::Go && search::go_type_group_member(&self.buf.lines, n - 1)
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
                && search::dart_label(&a.text, self.line + 1, range.start))
            || (kind == Kind::Solidity
                && search::solidity_key(&a.text, self.line + 1, before, after))
            || (kind == Kind::CSharp
                && !dotted
                && search::cs_initialized(&self.buf.lines, self.line, range.start, range.end)
                    .is_some());
        let super_keyword = !dotted && word == "super" && kind != Kind::Lua;
        let after = a.tail.trim_start();
        let rust_path = kind == Kind::Rust
            && (before.ends_with("::")
                || after.starts_with("::")
                || (after.starts_with('!') && !after.starts_with("!=")));
        let rust_field =
            kind == Kind::Rust && after.starts_with(':') && !after.starts_with("::") && !a.captured;
        let rust_type = kind == Kind::Rust
            && chain.is_empty()
            && first.starts_with(|c: char| c.is_ascii_uppercase())
            && !after.starts_with('.');
        let bare = kind == Kind::TsJs && !dotted && chain.is_empty();
        let required = match kind {
            Kind::TsJs => search::ts_import_lines1(&a.text, first),
            _ => Vec::new(),
        };
        let closure = |n: usize| {
            (kind == Kind::Swift
                || kind == Kind::Go && search::go_names_a_type_here(&a.lead, &a.tail)
                || kind == Kind::Python && search::python_in_function(&a.text, n)
                || kind == Kind::Jvm && search::scala(&a.here))
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
                        && (closure(n) || !self.declares_itself(kind, n, first) || bare)
                        && !import_line(kind, l)
                        && !rust_path
                        && !rust_type
                        && (!rust_field || n == self.line + 1)
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
        if let Some(own) = self.shell_line_locals(a) {
            return own;
        }
        if kind == Kind::Shell
            && locals.is_empty()
            && search::shell_command_position(&a.lead) == Some(false)
        {
            let lines: Vec<&str> = a.text.lines().collect();
            let literal = search::path_literal_lines(kind, &a.here, &a.text);
            locals.extend(search::shell_assigned(&lines, &literal, self.line, first));
        }
        if let [only] = locals[..]
            && bare
            && only != self.line + 1
        {
            locals.extend(search::ts_var_again(&a.text, only, first));
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
                || self.shell_line_locals(a).as_deref() == Some(locals)
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

    fn shell_line_locals(&self, a: &Asked) -> Option<Vec<usize>> {
        if a.kind != Kind::Shell {
            return None;
        }
        match search::shell_command_position(&a.lead) {
            Some(true) if !search::shell_assigns_word(&a.tail) => Some(Vec::new()),
            Some(false)
                if search::shell_binds_before(self.line_str(), a.range.start, a.first())
                    && search::path_literal_lines(a.kind, &a.here, &a.text).get(self.line)
                        != Some(&true) =>
            {
                Some(vec![self.line + 1])
            }
            _ => None,
        }
    }
}

fn rust_type_position(a: &Asked) -> bool {
    let (b, t) = (a.before.trim_end(), a.tail.trim_start());
    let marks = [
        ":", "->", "<", "&", "[", "mut", "const", "dyn", "impl", "for", "as", "where",
    ];
    let in_angles = b.matches('<').count() > b.matches('>').count() - b.matches("->").count();
    marks.iter().any(|m| {
        b.strip_suffix(m).is_some_and(|rest| {
            m.len() == 1 || !rest.ends_with(|c: char| c.is_alphanumeric() || c == '_')
        })
    }) || (b.ends_with(',') && in_angles)
        || t.starts_with("::")
        || (b.is_empty() && t.starts_with(':'))
}

fn ts_type_position(a: &Asked, line: &str) -> bool {
    let b = a.before.trim_end();
    let decl = line.trim_start();
    let decl = decl.strip_prefix("export ").unwrap_or(decl).trim_start();
    let decl = decl.strip_prefix("declare ").unwrap_or(decl).trim_start();
    let marks = [
        ":",
        "<",
        "|",
        "&",
        "extends",
        "implements",
        "keyof",
        "as",
        "satisfies",
        "is",
        "infer",
        "readonly",
    ];
    let in_angles = b.matches('<').count() > b.matches('>').count() - b.matches("=>").count();
    decl.starts_with("type ")
        || (b.ends_with(',') && in_angles)
        || marks.iter().any(|m| {
            b.strip_suffix(m).is_some_and(|rest| {
                m.len() == 1 || !rest.ends_with(|c: char| c.is_alphanumeric() || c == '_')
            })
        })
}

fn ts_key(a: &Asked, line1: usize) -> bool {
    let tail = a.tail.trim_start();
    a.kind == Kind::TsJs
        && !a.dotted
        && a.chain.is_empty()
        && a.lead.trim().is_empty()
        && tail.trim_start_matches('?').starts_with(':')
        && !tail.starts_with("::")
        && (search::ts_type_key(&a.text, line1, tail)
            || search::ts_type_literal_key(&a.text, line1 - 1, a.range.start))
}
