use super::{
    App, Candidate, FileLine, GoTypeUnread, Hit, Kind, Path, PathBuf, Reason, import_line,
    resolution, search,
};

pub(super) struct Asked {
    pub(super) kind: Kind,
    pub(super) here: PathBuf,
    pub(super) word: String,
    pub(super) range: std::ops::Range<usize>,
    pub(super) start: usize,
    pub(super) written: String,
    pub(super) before: String,
    pub(super) lead: String,
    pub(super) tail: String,
    pub(super) chain: Vec<String>,
    pub(super) text: String,
    pub(super) dotted: bool,
    pub(super) captured: bool,
    pub(super) go_keyed: bool,
}

impl Asked {
    pub(super) fn first(&self) -> &str {
        self.chain
            .first()
            .map_or(self.word.as_str(), String::as_str)
    }

    pub(super) fn own(&self) -> bool {
        matches!(self.chain.as_slice(), [s] if s == "self" || s == "cls" || s == "this")
    }

    pub(super) fn elixir_call(&self) -> bool {
        let after = self.tail.as_str();
        self.kind == Kind::Elixir
            && !self.dotted
            && (after.starts_with('(')
                || self.before.ends_with(['@', '&'])
                || self.before.trim_start().starts_with("@spec "))
    }
}

impl App {
    pub(super) fn show_found(&mut self, a: &Asked, found: Vec<Candidate>) -> bool {
        self.show_definitions(a.kind, &a.word, &a.here, found, None);
        true
    }

    pub(super) fn line_hit(&self, here: &Path, line: usize) -> Hit {
        Hit {
            deleted: None,
            path: here.to_path_buf(),
            line1: line,
            byte_col: None,
            text: self.buf.lines[line - 1].clone(),
        }
    }

    pub(super) fn lines_here(
        &self,
        a: &Asked,
        lines: impl IntoIterator<Item = usize>,
        reason: &Reason,
    ) -> Vec<Candidate> {
        lines
            .into_iter()
            .map(|line| Candidate {
                hit: self.line_hit(&a.here, line),
                reason: reason.clone(),
            })
            .collect()
    }

    pub(super) fn answered_without_word(&mut self, kind: Option<Kind>) -> bool {
        if kind == Some(Kind::Markdown)
            && let Some(here) = self.rel_current()
        {
            self.follow_markdown(&here);
            return true;
        }
        if self.probe.is_none()
            && let Some(FileLine { path, line1 }) = self.base_place()
        {
            self.definition_at_base(&path, line1);
            return true;
        }
        if let Some(here) = self.rel_current()
            && (self.follow_ref(&here)
                || self.css_definition(kind, &here)
                || kind == Some(Kind::Starlark) && self.starlark_definition(&here))
        {
            return true;
        }
        if let Some(kind) = kind
            && let Some(here) = self.rel_current()
            && let Some(module) = search::file_import(kind, self.line_str(), self.col)
        {
            let module = [module];
            let files = self.import_files(kind, &here, &module);
            let found = self.module_candidates(files);
            self.show_definitions(kind, &module[0], &here, found, None);
            return true;
        }
        false
    }

    pub(super) fn asked(&mut self, kind: Option<Kind>) -> Option<Asked> {
        let Some((range, mut word)) = self.definition_word(kind) else {
            self.message = "no word".into();
            return None;
        };
        let search::LineAsRead {
            line: written,
            word_start: start,
        } = self.written(kind, range.start);
        let (Some(before), Some(lead), Some(tail)) = (
            written.get(..start).map(str::to_owned),
            self.line_str().get(..range.start).map(str::to_owned),
            self.line_str().get(range.end..).map(str::to_owned),
        ) else {
            self.message = "no word".into();
            return None;
        };
        let dotted = before.ends_with('.') && !before.ends_with("..");
        let chain = search::qualifier(&written, start);
        let here = self.rel_current();
        let (Some(kind), Some(here)) = (kind, here) else {
            self.message = self.no_rules();
            return None;
        };
        if kind == Kind::Perl {
            self.perl_definition(&here);
            return None;
        }
        if self.component_template(&here, &range, &mut word)
            || (kind == Kind::Gdscript && self.gdscript_definition(&here, &range, &chain, &word))
        {
            return None;
        }
        if self.needs_no_lookup(kind, &word, &lead, range.start)
            || (kind == Kind::Shell
                && search::shell_no_name(&self.buf.lines, self.line, &lead, &tail))
        {
            self.message = resolution(&word, None, &[], None, false);
            return None;
        }
        let text = self.buf.lines.join("\n");
        let text = search::script_text(&here, &text, Some(self.line)).into_owned();
        let at = self.buf.lines[..self.line]
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + range.start;
        let captured = lead.ends_with('{') && !lead.ends_with("{{") && tail.starts_with(['}', ':']);
        if !captured
            && search::in_string(kind, &text, at)
            && !search::rust_attribute_path(&self.buf.lines, self.line, range.start)
        {
            self.message = resolution(&word, None, &[], None, false);
            return None;
        }
        Some(Asked {
            kind,
            here,
            word,
            range,
            start,
            written,
            before,
            lead,
            tail,
            chain,
            text,
            dotted,
            captured,
            go_keyed: false,
        })
    }

    fn needs_no_lookup(&self, kind: Kind, word: &str, lead: &str, start: usize) -> bool {
        (kind == Kind::Go && word == "_")
            || (kind == Kind::Graphql && lead.ends_with('$'))
            || (kind == Kind::Jvm && word == "class" && lead.ends_with("::"))
            || (kind == Kind::PowerShell && search::powershell_argument(lead))
            || (kind == Kind::R && search::r_foreign_package(self.line_str(), start, &self.root))
    }

    pub(super) fn answered_early(&mut self, a: &mut Asked) -> bool {
        if self.answered_by_module(a) {
            return true;
        }
        if a.kind == Kind::Elixir {
            a.chain = search::elixir_unalias(&a.text, self.line, std::mem::take(&mut a.chain));
        }
        self.answered_by_file(a)
            || self.answered_in_file(a)
            || self.answered_by_go_key(a)
            || self.answered_by_sigil(a)
    }

    fn answered_by_module(&mut self, a: &Asked) -> bool {
        let kind = a.kind;
        if let Some(found) = self.ml_definitions(kind, &a.here, &a.chain, &a.word, a.dotted) {
            return self.show_found(a, found);
        }
        if kind == Kind::Jvm
            && import_line(kind, self.line_str())
            && !a.lead.contains('{')
            && let Some(found) =
                self.jvm_imported(&a.text, &a.chain, &a.word, a.range.clone(), true)
        {
            return self.show_found(a, found);
        }
        if kind == Kind::Jvm && self.android_definition(&a.here, &a.text, &a.word, a.range.start) {
            return true;
        }
        let early = match kind {
            Kind::CSharp => self.cs_namespace_segment(&a.here, &a.range),
            Kind::Elixir => self.erlang_definitions(&a.here, &a.word, a.range.clone()),
            _ => None,
        };
        if let Some(found) = early {
            return self.show_found(a, found);
        }
        false
    }

    fn answered_by_file(&mut self, a: &Asked) -> bool {
        let kind = a.kind;
        if kind == Kind::Yaml
            && let Some(lines) = search::stage_entries(a.lead.as_str(), &a.text, &a.word)
        {
            let found = self.lines_here(a, lines, &Reason::ByName);
            return self.show_found(a, found);
        }
        if kind == Kind::Proto
            && let Some(found) = self.proto_definitions(&a.text, &a.before, &a.word)
        {
            return self.show_found(a, found);
        }
        if let Some(found) =
            self.file_imported(kind, &a.here, &a.text, &a.before, &a.chain, &a.word)
        {
            return self.show_found(a, found);
        }
        if self.probe.is_none()
            && let java = a.here.extension().is_some_and(|e| e == "java")
            && let Some(label) = search::label_at(kind, java, &a.text, self.line, a.range.clone())
        {
            let found = label
                .owner
                .map(|o| self.label_targets(kind, &a.here, &a.word, o.line0, o.byte_col, &o.how))
                .unwrap_or_default();
            if found.is_empty() {
                self.message = format!("{}: {}", a.word, label.what);
            } else {
                self.show_found(a, found);
            }
            return true;
        }
        false
    }

    fn answered_in_file(&mut self, a: &Asked) -> bool {
        let kind = a.kind;
        self.offer_only =
            kind == Kind::Python && search::names_a_parameter(&a.text, self.line + 1, &a.range);
        self.truncated.set(false);
        let sigil = a.lead.as_str();
        if kind == Kind::Php
            && sigil.ends_with('$')
            && !sigil.ends_with("::$")
            && a.word != "this"
            && let Some(lines) = search::php_variable(&a.text, self.line + 1, &a.word)
        {
            let found = self.lines_here(a, lines, &Reason::Local);
            return self.show_found(a, found);
        }
        if kind == Kind::Sql && !a.dotted {
            let at = self.buf.lines[..self.line]
                .iter()
                .map(|l| l.len() + 1)
                .sum::<usize>()
                + a.range.start;
            let lines = search::sql_local(&a.text, at, &a.word);
            if !lines.is_empty() {
                let found = self.lines_here(a, lines, &Reason::Local);
                return self.show_found(a, found);
            }
        }
        if kind == Kind::C
            && let Some((found, broke)) = self.c_early(&a.here, &a.text, &a.word, a.range.clone())
        {
            self.show_definitions(kind, &a.word, &a.here, found, broke.as_deref());
            return true;
        }
        if kind == Kind::TsJs && matches!(a.chain.join(".").as_str(), "exports" | "module.exports")
        {
            let lines = search::cjs_exported_lines(&a.text, &a.word);
            if !lines.is_empty() {
                let found = self.lines_here(a, lines, &Reason::Path(a.chain.join(".")));
                return self.show_found(a, found);
            }
        }
        if kind == Kind::Python
            && let Some(module) = search::python_import_module(self.line_str(), a.range.start)
        {
            let found = self.python_module(&a.here, &module);
            return self.show_found(a, found);
        }
        false
    }

    fn answered_by_go_key(&mut self, a: &mut Asked) -> bool {
        let (kind, dotted, range) = (a.kind, a.dotted, &a.range);
        let go_key = if kind == Kind::Go && !dotted {
            search::go_key(&a.text, self.line + 1, range.start, range.end)
        } else {
            search::GoKey::No
        };
        a.go_keyed = go_key != search::GoKey::No;
        if kind == Kind::Go && (dotted || matches!(go_key, search::GoKey::Of(_))) {
            self.external_files(kind);
        }
        match go_key {
            search::GoKey::Of(written) => {
                match self.literal_field(kind, &a.here, &written, &a.word) {
                    Ok(Some(found)) => return self.show_found(a, found),
                    Ok(None) => {}
                    Err(GoTypeUnread) => self.offer_only = true,
                }
            }
            search::GoKey::Struct(line) => {
                let ty = super::typed::anonymous(&a.here, line);
                let found = self
                    .field_of(kind, &ty, &a.word, false)
                    .into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Receiver(ty.name.clone()),
                    })
                    .collect();
                return self.show_found(a, found);
            }
            search::GoKey::Unknown => self.offer_only = true,
            search::GoKey::No | search::GoKey::Value => {}
        }
        false
    }

    fn answered_by_sigil(&mut self, a: &Asked) -> bool {
        let lead = a.lead.as_str();
        if a.kind == Kind::Ruby && lead.ends_with('@') {
            let sigil = if lead.ends_with("@@") { "@@" } else { "@" };
            let ivar = format!("{sigil}{}", a.word);
            let found = self.ruby_ivars(&a.here, &a.text, &ivar);
            self.show_definitions(a.kind, &ivar, &a.here, found, None);
            return true;
        }
        if a.kind == Kind::Clojure
            && let Some(found) =
                self.clojure_qualified(&a.text, &a.before, &a.word, a.tail.as_str())
        {
            return self.show_found(a, found);
        }
        false
    }
}
