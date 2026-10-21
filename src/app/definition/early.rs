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
            && let Some(module) = search::file_import(kind, &here, self.line_str(), self.col)
        {
            let module = [module];
            let files = self.import_files(kind, &here, &module);
            if kind != Kind::Shell || !files.is_empty() {
                let found = self.module_candidates(files);
                self.show_definitions(kind, &module[0], &here, found, None);
                return true;
            }
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
        let captured = kind == Kind::Rust
            && lead.ends_with('{')
            && !lead.ends_with("{{")
            && tail.starts_with(['}', ':']);
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
            || (kind == Kind::Ruby
                && search::ruby_loop_keyword(
                    word,
                    lead,
                    self.line_str()
                        .get(start + word.len()..)
                        .unwrap_or_default(),
                ))
            || (kind == Kind::R && search::r_foreign_package(self.line_str(), start, &self.root))
    }

    pub(super) fn answered_early(&mut self, a: &mut Asked) -> bool {
        if self.answered_by_module(a) || self.answered_on_rust_field(a) {
            return true;
        }
        if a.kind == Kind::Elixir {
            a.chain = search::elixir_unalias(&a.text, self.line, std::mem::take(&mut a.chain));
        }
        if a.kind == Kind::Ruby
            && a.dotted
            && a.chain == ["self", "class"]
            && search::ruby_instance_of_class(&a.text, self.line + 1)
        {
            let class = search::ruby_class_path(&a.text, self.line + 1);
            a.chain = class.split("::").map(str::to_owned).collect();
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

    fn answered_on_rust_field(&mut self, a: &Asked) -> bool {
        static VISIBILITY: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?$").expect("the regex is a literal")
        });
        let line1 = self.line + 1;
        let on_field = a.kind == Kind::Rust
            && !a.dotted
            && VISIBILITY.is_match(&a.lead)
            && a.tail.trim_start().starts_with(':')
            && !a.tail.trim_start().starts_with("::")
            && search::rust_field_at(&a.text, line1);
        if !on_field {
            return false;
        }
        let methods = search::rust_method_pattern(&a.word);
        let fields = search::rust_field_pattern(&a.word, false);
        let mut hits = self.project_definitions(a.kind, &a.here, &a.word, &methods);
        hits.extend(
            self.project_definitions(a.kind, &a.here, &a.word, &fields)
                .into_iter()
                .filter(|h| {
                    self.text_of(&h.path)
                        .is_some_and(|t| search::rust_field_at(&t, h.line1))
                }),
        );
        if !hits.iter().any(|h| h.path == a.here && h.line1 == line1) {
            hits.push(self.line_hit(&a.here, line1));
        }
        let found = hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect();
        self.show_found(a, found)
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
            && ((sigil.ends_with('$') && !sigil.ends_with("::$")) || sigil.ends_with("${"))
            && a.word != "this"
            && let Some(lines) = search::php_variable(&a.text, self.line + 1, &a.word)
        {
            let found = self.lines_here(a, lines, &Reason::Local);
            return self.show_found(a, found);
        }
        if kind == Kind::Lua
            && let Some(lines) = search::lua_goto_label(&a.text, self.line + 1, &a.lead, &a.word)
        {
            let found = self.lines_here(a, lines, &Reason::Local);
            return self.show_found(a, found);
        }
        if kind == Kind::Sql
            && let Some(shown) = self.sql_in_file(a)
        {
            return shown;
        }
        if kind == Kind::C
            && let Some((found, broke)) = self.c_early(&a.here, &a.text, &a.word, a.range.clone())
        {
            self.show_definitions(kind, &a.word, &a.here, found, broke.as_deref());
            return true;
        }
        if kind == Kind::TsJs && matches!(a.chain.join(".").as_str(), "exports" | "module.exports")
        {
            let literal = search::literal_lines(kind, &a.text);
            let lines: Vec<usize> = search::cjs_exported_lines(&a.text, &a.word)
                .into_iter()
                .filter(|&n| literal.get(n - 1) != Some(&true))
                .collect();
            if !lines.is_empty() {
                let found = self.lines_here(a, lines, &Reason::Path(a.chain.join(".")));
                return self.show_found(a, found);
            }
        }
        if search::reads_jsdoc(kind, &a.here)
            && a.chain.is_empty()
            && !a.dotted
            && search::jsdoc_braced_at(
                self.line_str(),
                a.range.clone(),
                search::literal_lines(kind, &a.text).get(self.line) == Some(&true),
            )
        {
            let found: Vec<Hit> = match &search::jsdoc_typedefs(&a.text, &a.word)[..] {
                [] => self
                    .type_decl(kind, &a.here, &a.word)
                    .map(|ty| self.export_hit(&ty.path, ty.line))
                    .into_iter()
                    .collect(),
                typedefs => typedefs
                    .iter()
                    .map(|t| self.line_hit(&a.here, t.line1))
                    .collect(),
            };
            if !found.is_empty() {
                let found = found
                    .into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::Path("JSDoc".to_owned()),
                    })
                    .collect();
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
        if kind == Kind::Go
            && (dotted || matches!(go_key, search::GoKey::Of(_) | search::GoKey::ElementOf(_)))
        {
            self.external_files(kind);
        }
        let (go_key, at) = match go_key {
            search::GoKey::ElementOf(named) => match self.named_element(kind, &a.here, &named) {
                Some((element, at)) => {
                    let element = element.trim_start_matches('*').to_owned();
                    (search::GoKey::Of(element), at)
                }
                None => (search::GoKey::Unknown, a.here.clone()),
            },
            key => (key, a.here.clone()),
        };
        match go_key {
            search::GoKey::Of(written) => match self.literal_field(kind, &at, &written, &a.word) {
                Ok(Some(found)) => return self.show_found(a, found),
                Ok(None) => {}
                Err(GoTypeUnread) => self.offer_only = true,
            },
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
            search::GoKey::No | search::GoKey::Value | search::GoKey::ElementOf(_) => {}
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

impl App {
    fn sql_in_file(&mut self, a: &Asked) -> Option<bool> {
        let at = self.buf.lines[..self.line]
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + a.range.start;
        let quoted = at > 0 && a.text.as_bytes().get(at - 1) == Some(&b'"');
        let (table, lines) = if a.dotted {
            let [qualifier] = a.chain.as_slice() else {
                return None;
            };
            (search::sql_alias_table(&a.text, at, qualifier), Vec::new())
        } else {
            let (lines, column_wins) = search::sql_local(&a.text, at, &a.word);
            let table = (lines.is_empty() || column_wins)
                .then(|| search::sql_bare_table(&a.text, at, &a.word))
                .flatten();
            (table, lines)
        };
        let found = table
            .map(|t| self.sql_column(&a.here, &t, (&a.word, quoted), 4))
            .unwrap_or_default();
        if found.is_empty() && !lines.is_empty() {
            let found = self.lines_here(a, lines, &Reason::Local);
            return Some(self.show_found(a, found));
        }
        (!found.is_empty()).then(|| self.show_found(a, found))
    }

    fn sql_column(
        &self,
        here: &Path,
        table: &str,
        asked: (&str, bool),
        depth: usize,
    ) -> Vec<Candidate> {
        let (column, quoted) = asked;
        let mut hits =
            self.project_grep(Kind::Sql, here, &search::sql_column_pattern(table, column));
        let head = regex::Regex::new(r"(?i)^\s*(?:CREATE|ALTER)\s").expect("a valid pattern");
        hits.retain(|h| {
            head.is_match(&h.text)
                || self
                    .hit_text(h)
                    .is_some_and(|t| search::sql_alters(&t, h.line1, table))
        });
        let tabled: Vec<Hit> = hits
            .iter()
            .map(|h| Hit {
                text: format!("TABLE {table}"),
                ..h.clone()
            })
            .collect();
        let keep = search::sql_in_effect(here, self.line + 1, &tabled);
        let mut i = 0;
        hits.retain(|h| {
            i += 1;
            keep.as_ref()
                .map_or_else(|| !h.path.ends_with("down.sql"), |k| k.contains(&(i - 1)))
        });
        let create = regex::Regex::new(r"(?i)^\s*CREATE\s").expect("a valid pattern");
        let mut parents = Vec::new();
        let found: Vec<Candidate> = hits
            .into_iter()
            .filter_map(|h| {
                if !create.is_match(&h.text) {
                    return Some(h);
                }
                let whole = self.hit_text(&h)?;
                let Some(line1) = search::sql_table_column(&whole, h.line1, column, quoted) else {
                    parents.extend(search::sql_parents(&whole, h.line1));
                    return None;
                };
                let text = whole.lines().nth(line1 - 1)?.to_owned();
                Some(Hit {
                    line1,
                    text,
                    byte_col: None,
                    ..h
                })
            })
            .map(|hit| Candidate {
                hit,
                reason: Reason::Receiver(table.to_owned()),
            })
            .collect();
        if !found.is_empty() || depth == 0 {
            return found;
        }
        parents.sort();
        parents.dedup();
        parents
            .iter()
            .filter(|p| *p != table)
            .flat_map(|p| self.sql_column(here, p, asked, depth - 1))
            .collect()
    }
}
