use super::{
    App, Asked, Candidate, HashMap, Hit, Imported, Kind, Near, PathBuf, Patterns, Reason, Walk,
    enclosing_def_lines1, external, is_word, search, starts_by_assigning,
};

impl App {
    pub(super) fn define_by_name(
        &mut self,
        a: &Asked,
        near: &Near,
        p: &Patterns,
        walk: &Walk,
        imported: &Imported,
    ) {
        let kind = a.kind;
        let hits = self.hits_by_name(a, near, p, walk);
        let mut fallbacks = Vec::new();
        let mut aside = None;
        let hits = if kind == Kind::C && !a.dotted && !a.before.ends_with("->") {
            let own = search::cpp_constructor_defined_in_class(
                &a.text,
                self.line + 1,
                a.range.start,
                &a.word,
            );
            let delegated = (!own && a.tail.trim_start().starts_with('('))
                .then(|| {
                    search::cpp_delegated_constructor(
                        &a.text,
                        self.line + 1,
                        a.range.start,
                        &a.word,
                        || self.c_included_texts(&a.here),
                    )
                })
                .flatten();
            let Some(hits) = (match delegated {
                _ if own => Some(vec![self.own_hit(a)]),
                Some((file, line)) => Some(vec![self.c_delegated_hit(a, file, line)]),
                None => self.c_hits(a, near, hits, &mut fallbacks, &mut aside),
            }) else {
                return;
            };
            hits
        } else {
            hits
        };
        let mut found: Vec<Candidate> = hits
            .into_iter()
            .map(|hit| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect();
        if let Some(module) = &imported.value {
            found.retain(|c| starts_by_assigning(&c.hit.text, &a.word));
            found.extend(self.block_assignments(&a.here, &a.word));
            if found.is_empty() {
                found = self.package_assignments(&a.word, module);
            }
        }
        if kind == Kind::Python && near.unbound {
            self.star_winner(&a.here, &a.text, &a.word, &mut found);
        }
        if kind == Kind::Swift && !found.is_empty() {
            self.swift_extensions(a, near, &mut found);
            self.swift_init_by_labels(&a.here, &a.word, a.range.end, &mut found);
            if a.dotted && a.chain.is_empty() && self.swift_one_owner(&a.word, &found) {
                self.swift_by_labels(&a.word, a.range.end, &mut found);
            }
        }
        if kind == Kind::Dart {
            let member = a.before.trim_end().ends_with('.');
            self.dart_reachable(&a.here, member, &mut found);
            if !member {
                self.dart_visible(&a.here, &a.word, &a.tail, &mut found);
            }
        }
        if kind == Kind::TsJs && a.dotted {
            self.ts_member_namesakes(a, near, &mut found);
        }
        self.found_outside(a, near, p, imported, &mut found);
        if kind == Kind::Dart && !a.before.trim_end().ends_with('.') {
            self.dart_unimported(&a.here, &a.word, &mut found);
            self.dart_outside_constructor(&a.word, &a.tail, &mut found);
        }
        self.found_last(a, p, walk, fallbacks, &mut found);
        if kind == Kind::Jvm && a.dotted {
            self.jvm_by_arguments(&a.here, &a.word, a.range.end, &mut found);
        }
        if kind == Kind::Rust && found.is_empty() {
            found = self.rust_macro_declared(&a.word);
        }
        if kind == Kind::Ruby && p.on_value {
            self.ruby_member_outside(&a.word, &p.pattern, &mut found);
        }
        if kind == Kind::Ruby
            && !a.dotted
            && a.chain.is_empty()
            && !a.before.ends_with("::")
            && a.word
                .starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
        {
            self.offer_only |= self.ruby_all_unrelated(&a.here, &a.text, &found);
        }
        self.show_definitions(kind, &a.word, &a.here, found, walk.broke.as_deref());
        if let Some(note) = aside {
            self.message = self.message.replacen("1 match", &note, 1);
        }
    }

    fn ts_member_namesakes(&mut self, a: &Asked, near: &Near, found: &mut Vec<Candidate>) {
        static TOP_LEVEL: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(
                r"^(?:export\s+)?(?:default\s+)?(?:declare\s+)?(?:abstract\s+)?(?:async\s+)?(?:function\*?|class)\s",
            )
            .expect("the regex is a literal")
        });
        found.retain(|c| {
            !self.text_of(&c.hit.path).is_some_and(|t| {
                search::ts_nested_local(&t.lines().collect::<Vec<_>>(), c.hit.line1)
            })
        });
        let namespace = a.chain.first().is_some_and(|head| {
            matches!(head.as_str(), "exports" | "module")
                || near.imports.iter().any(|i| {
                    i.name == *head
                        && matches!(i.path.last().map(String::as_str), Some("*" | "default"))
                })
        });
        if !namespace && found.iter().any(|c| TOP_LEVEL.is_match(&c.hit.text)) {
            self.offer_only = true;
        }
    }

    fn found_last(
        &mut self,
        a: &Asked,
        p: &Patterns,
        walk: &Walk,
        fallbacks: Vec<Hit>,
        found: &mut Vec<Candidate>,
    ) {
        let kind = a.kind;
        if kind == Kind::Zig {
            found.retain(|c| {
                (c.hit.path == a.here && c.hit.line1 == self.line + 1)
                    || self
                        .text_of(&c.hit.path)
                        .is_some_and(|t| search::zig_visible(&t, c.hit.line1, c.hit.path == a.here))
            });
        }
        if found.is_empty() {
            *found = fallbacks
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect();
        }
        if kind == Kind::Jvm && a.dotted && !found.is_empty() && a.chain.is_empty() {
            let fields = self.jvm_lombok(&a.here, &a.text, &a.chain, &a.word);
            found.extend(fields);
        }
        if kind == Kind::Jvm && found.is_empty() {
            *found = self.jvm_lombok(&a.here, &a.text, &a.chain, &a.word);
            if found.is_empty() && !a.dotted {
                *found = self.jenkins_step(&a.here, &a.word);
            }
        }
        if found.is_empty() && self.probe.is_none() {
            *found = self.deleted_definitions(kind, &a.word, &a.here, p.cs_type);
            if kind == Kind::CSharp && !found.is_empty() {
                let chain = a.dotted.then_some(a.chain.as_slice());
                let hits = found.drain(..).map(|c| c.hit).collect();
                *found =
                    (self.cs_reachable(&a.here, &a.word, chain, walk.cs_walked.as_deref(), hits))
                        .into_iter()
                        .map(|hit| Candidate {
                            hit,
                            reason: Reason::ByName,
                        })
                        .collect();
            }
        }
    }

    fn swift_extensions(&mut self, a: &Asked, near: &Near, found: &mut Vec<Candidate>) {
        let extends = |c: &Candidate| search::swift_extension(&c.hit.text);
        if found.iter().any(|c| !extends(c)) {
            found.retain(|c| !extends(c));
        } else {
            let mut away = self
                .external_definitions(a.kind, &a.word, &a.chain, a.dotted, &near.imports, false)
                .unwrap_or_default();
            away.retain(|c| !extends(c));
            if away.is_empty() {
                self.offer_only = true;
            } else {
                *found = away;
            }
        }
    }

    fn found_outside(
        &mut self,
        a: &Asked,
        near: &Near,
        p: &Patterns,
        imported: &Imported,
        found: &mut Vec<Candidate>,
    ) {
        let (kind, word) = (a.kind, &a.word);
        if let Some(members) = &p.members {
            let all = self.external_files(kind);
            let files: Vec<PathBuf> = all
                .iter()
                .filter(|p| kind != Kind::TsJs || search::declaration_file(p))
                .cloned()
                .collect();
            let external::OutsideMembers {
                methods: external,
                unlisted_field_declares_it,
            } = self.outside_members(kind, &all, &files, &near.imports, members, word);
            if found.is_empty() && external.len() == 1 && unlisted_field_declares_it {
                self.truncated.set(true);
            }
            let external = if kind == Kind::TsJs && found.is_empty() && external.is_empty() {
                let signature = search::member_or_signature(kind, word)
                    .and_then(|patterns| patterns.last().cloned());
                let properties = signature
                    .map(|s| self.outside_grep(kind, &files, &s, None))
                    .unwrap_or_default();
                self.offer_only |= !properties.is_empty();
                properties
            } else {
                external
            };
            let seen: Vec<PathBuf> = found.iter().map(|c| self.root.join(&c.hit.path)).collect();
            found.extend(
                external
                    .into_iter()
                    .filter(|h| !seen.contains(&h.path))
                    .map(|hit| Candidate {
                        hit,
                        reason: Reason::ByName,
                    }),
            );
        } else if found.is_empty()
            && !imported.outside
            && imported.value.is_none()
            && !matches!(
                a.chain.first().map(String::as_str),
                Some("self" | "cls" | "this")
            )
        {
            *found = if near.unbound && !search::PYTHON_BUILTINS.contains(&word.as_str()) {
                self.star_imported(word, &near.imports)
            } else {
                self.external_definitions(kind, word, &a.chain, a.dotted, &near.imports, false)
                    .unwrap_or_default()
            };
            let after = a.tail.trim_start();
            if kind == Kind::C
                && near.locals.is_empty()
                && (after.starts_with("->") || (after.starts_with('.') && !after.starts_with("..")))
            {
                found.retain(|c| !Self::c_type_line(word, &c.hit));
            }
            if imported.linked {
                for c in found.iter_mut() {
                    c.reason = Reason::ByName;
                }
            }
        }
    }

    fn hits_by_name(&mut self, a: &Asked, near: &Near, p: &Patterns, walk: &Walk) -> Vec<Hit> {
        let (kind, dotted, chain, word) = (a.kind, a.dotted, &a.chain, &a.word);
        let mut hits = p
            .members
            .as_ref()
            .map(|m| self.members_by_name(kind, &a.here, word, m))
            .filter(|hits| !hits.is_empty() || !near.locals.is_empty() || kind == Kind::Php)
            .unwrap_or_else(|| {
                if a.own() {
                    let pattern = p.php_members.as_ref().unwrap_or(&p.pattern);
                    self.members_by_name(kind, &a.here, word, pattern)
                } else {
                    self.project_definitions(kind, &a.here, word, &p.pattern)
                }
            });
        if kind == Kind::Python
            && !dotted
            && chain.is_empty()
            && !walk.declares_here
            && !search::python_class_binds(&a.text, self.line + 1, word)
        {
            self.python_reachable(a, near, &mut hits);
        }
        self.swift_implicit(a, &mut hits);
        if kind == Kind::Swift
            && dotted
            && chain.first().is_some_and(|f| f != "self" && f != "super")
            && chain
                .iter()
                .all(|n| n.starts_with(|c: char| c.is_lowercase() || c.is_ascii_digit()))
        {
            hits.retain(|h| {
                !search::swift_case(&h.text)
                    && search::swift_member_kind(&h.text)
                        != Some(search::SwiftMember::StaticOrClass)
            });
        }
        if kind == Kind::Swift && !dotted && chain.is_empty() {
            self.swift_nested_unseen(&mut hits);
        }
        if p.ruby_local && !dotted && chain.is_empty() {
            hits.extend(
                search::ruby_locals(&a.text, self.line + 1, word)
                    .into_iter()
                    .map(|line| self.line_hit(&a.here, line)),
            );
        }
        if kind == Kind::Ruby
            && !dotted
            && chain.is_empty()
            && !a.before.ends_with("::")
            && word.starts_with(|c: char| c.is_ascii_uppercase())
        {
            self.ruby_constant_in_scope(&a.here, &a.text, word, &mut hits);
        }
        if kind == Kind::CSharp {
            let chain = dotted.then_some(chain.as_slice());
            hits = self.cs_reachable(&a.here, word, chain, walk.cs_walked.as_deref(), hits);
        }
        if kind == Kind::Lua {
            self.lua_hits(a, &mut hits);
        }
        if near.arrow_param
            && !hits
                .iter()
                .any(|h| h.path == a.here && h.line1 == self.line + 1)
        {
            hits.push(self.own_hit(a));
        }
        if kind == Kind::Swift {
            self.swift_scoped(a, near, &mut hits);
        }
        match kind {
            Kind::Jvm => self.jvm_hits(a, hits),
            _ => hits,
        }
    }

    fn lua_hits(&mut self, a: &Asked, hits: &mut Vec<Hit>) {
        let word = &a.word;
        let at = |h: &Hit| h.path == a.here && h.line1 == self.line + 1;
        let bare = !a.dotted && a.chain.is_empty();
        let narrow = bare && !lua_qualified_elsewhere(&a.here, word, hits);
        hits.retain(|h| {
            let t = h.text.trim_start();
            !bare
                || h.path == a.here
                || !((narrow && t.starts_with("local "))
                    || (t.starts_with(word.as_str()) && self.lua_key_elsewhere(h, word)))
        });
        let all = hits.len();
        let unseen = bare
            .then(|| search::lua_value_of_its_own(&a.text, self.line + 1, word))
            .flatten();
        hits.retain(|h| {
            (!h.text.starts_with([' ', '\t'])
                || !h.text.trim_start().starts_with("local ")
                || at(h))
                && (h.path != a.here || Some(h.line1) != unseen)
                && !(bare
                    && h.path == a.here
                    && h.line1 > self.line + 1
                    && h.text.trim_start().starts_with("local "))
        });
        self.offer_only |= hits.len() < all && hits.iter().any(|h| !at(h));
        if let Some(lib) = search::lua_std_receiver(&a.text, self.line + 1, &a.before) {
            let (l, w) = (regex::escape(lib), regex::escape(word));
            let patched = regex::Regex::new(&format!(
                r"^\s*(?:function\s+{l}[.:]{w}\s*\(|{l}\.{w}\s*=[^=])"
            ));
            hits.retain(|h| patched.as_ref().is_ok_and(|re| re.is_match(&h.text)));
        }
    }

    fn lua_key_elsewhere(&self, h: &Hit, word: &str) -> bool {
        let start = h.text.len() - h.text.trim_start().len();
        std::fs::read_to_string(self.root.join(&h.path))
            .is_ok_and(|text| search::table_key(&text, h.line1, &(start..start + word.len())))
    }

    fn own_hit(&self, a: &Asked) -> Hit {
        Hit {
            path: a.here.clone(),
            line1: self.line + 1,
            byte_col: None,
            text: self.line_str().to_owned(),
            deleted: None,
        }
    }

    fn python_reachable(&self, a: &Asked, near: &Near, hits: &mut Vec<Hit>) {
        let all = hits.len();
        let mut kept = hits.clone();
        kept.retain(|h| {
            let d = h.text.trim_start();
            !(d.starts_with("def ") || d.starts_with("async def "))
                || !self
                    .text_of(&h.path)
                    .is_some_and(|t| search::python_method(&t, h.line1))
        });
        let around = enclosing_def_lines1(&a.text, self.line + 1);
        let unreachable = |h: &Hit| {
            let inside = self
                .text_of(&h.path)
                .and_then(|t| enclosing_def_lines1(&t, h.line1).first().copied());
            inside.is_some_and(|f| h.path != a.here || !around.contains(&f))
        };
        let global = |h: &Hit| h.path == a.here && !h.text.starts_with(char::is_whitespace);
        if kept.iter().any(global)
            && !near
                .imports
                .iter()
                .any(|i| i.name == a.word || i.name == "*")
        {
            kept.retain(|h| h.path == a.here);
        }
        if !matches!(kept.as_slice(), [h] if all > 1 && unreachable(h)) {
            *hits = kept;
        }
    }

    fn swift_implicit(&self, a: &Asked, hits: &mut Vec<Hit>) {
        if a.kind == Kind::Swift
            && a.dotted
            && a.chain.is_empty()
            && a.word != "init"
            && let Some(dot) = a.before.strip_suffix('.')
        {
            let lead = dot.trim_end();
            let implicit = lead.is_empty()
                || lead.ends_with([
                    '(', ',', '[', ':', '=', '<', '>', '&', '|', '+', '-', '*', '/', '%', '^', '~',
                ])
                || (lead.len() < dot.len() && lead.ends_with(['?', '!']))
                || ["return", "case", "let", "var"]
                    .iter()
                    .any(|k| lead.strip_suffix(k).is_some_and(|b| !b.ends_with(is_word)));
            let pattern = a.written.trim_start().starts_with("case ");
            let kept: Vec<Hit> = hits
                .iter()
                .filter(|h| {
                    search::swift_case(&h.text)
                        || (!pattern
                            && search::swift_member_kind(&h.text)
                                == Some(search::SwiftMember::StaticOrClass))
                })
                .cloned()
                .collect();
            if implicit && !kept.is_empty() {
                *hits = kept;
            } else if implicit && !(lead.is_empty() && search::swift_continues(&a.text, self.line))
            {
                hits.retain(|h| !search::swift_instance_member(&h.text));
            }
        }
    }

    fn swift_scoped(&mut self, a: &Asked, near: &Near, hits: &mut Vec<Hit>) {
        let own = self.own_hit(a);
        if near.swift_binds_here
            && !hits
                .iter()
                .any(|h| h.path == own.path && h.line1 == own.line1)
        {
            hits.push(own);
        }
        let all = hits.len();
        let lines: Vec<&str> = a.text.lines().collect();
        let literal = search::literal_lines(a.kind, &a.text);
        let mut scopes = vec![search::swift_scope(&lines, &literal, self.line + 1).header_line1];
        while let Some(&s @ 1..) = scopes.last() {
            match search::swift_scope(&lines, &literal, s).header_line1 {
                0 => break,
                up => scopes.push(up),
            }
        }
        let mut files: HashMap<PathBuf, (Vec<String>, Vec<bool>)> = HashMap::new();
        hits.retain(|h| {
            let (lines, literal) = files.entry(h.path.clone()).or_insert_with(|| {
                let t = self.text_of(&h.path).unwrap_or_default();
                let literal = search::literal_lines(a.kind, &t);
                (t.lines().map(str::to_owned).collect(), literal)
            });
            match search::swift_local(lines, literal, h.line1) {
                Some(at) => !a.dotted && h.path == a.here && scopes.contains(&at),
                None => true,
            }
        });
        if !a.dotted && search::swift_in_top_level_func(&lines, &literal, self.line + 1) {
            let in_type = |h: &Hit| {
                files.get(&h.path).is_some_and(|(lines, literal)| {
                    search::swift_enclosing_type(lines, literal, h.line1).is_some()
                })
            };
            let top = |h: &Hit| {
                !in_type(h)
                    && !h.text.starts_with([' ', '\t'])
                    && (h.path != a.here || h.line1 != self.line + 1)
            };
            if hits.iter().any(top) {
                hits.retain(|h| !in_type(h));
            }
        }
        self.offer_only |= hits.len() < all
            && matches!(hits.as_slice(), [h] if h.path == a.here && h.line1 == self.line + 1);
    }

    fn jvm_hits(&mut self, a: &Asked, hits: Vec<Hit>) -> Vec<Hit> {
        let hits = self.jvm_seen(&a.here, hits, a.dotted || a.before.ends_with("::"));
        let hits = self.jvm_use_fit(&a.here, &a.word, a.range.clone(), &a.chain, hits);
        if matches!(a.word.as_str(), "values" | "valueOf")
            && matches!(hits.as_slice(), [h] if h.text.contains("enum "))
        {
            self.offer_only = false;
        }
        hits
    }

    fn c_hits(
        &mut self,
        a: &Asked,
        near: &Near,
        hits: Vec<Hit>,
        fallbacks: &mut Vec<Hit>,
        aside: &mut Option<String>,
    ) -> Option<Vec<Hit>> {
        let word = &a.word;
        let (before, after) = (a.before.trim_end(), a.tail.as_str());
        let keyword = |k: &str| {
            before
                .strip_suffix(k)
                .is_some_and(|b| !b.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
        };
        let construction = after.trim_start().starts_with(['(', '{']) || keyword("new");
        let after = after.trim_start();
        let value = near.locals.is_empty()
            && (after.starts_with("->") || (after.starts_with('.') && !after.starts_with("..")));
        let tag = ["struct", "union", "enum"].into_iter().any(keyword);
        let mut literal = HashMap::new();
        let hits: Vec<Hit> = hits
            .into_iter()
            .filter(|h| !value || !Self::c_type_line(word, h))
            .filter(|h| self.c_code_line(&mut literal, h))
            .collect();
        let hits = if self.c_source() {
            c_namespace(word, hits, tag, after.starts_with('('))
        } else {
            hits
        };
        let hits = search::c_type_rows(
            word,
            hits,
            |p| self.text_of(p),
            search::CTypeWord {
                constructs: construction,
                struct_tag: tag,
            },
        );
        let hits: Vec<Hit> = (self.c_reached_only(&a.here, word, hits).into_iter())
            .filter(|h| {
                h.path == a.here
                    || !Self::c_type_line(word, h)
                    || !(self.text_of(&h.path))
                        .is_some_and(|t| search::c_inside_function(&t, h.line1, word))
            })
            .collect();
        let on = hits
            .iter()
            .any(|h| h.path == a.here && h.line1 == self.line + 1);
        let hits = {
            let by_macro = self.c_static_by_macro(&a.here, &hits);
            search::c_file_local(
                word,
                &a.here,
                &a.text,
                hits,
                &by_macro,
                |p| self.text_of(p),
                on,
            )
        };
        let hits = if a.chain.is_empty()
            && near.locals.is_empty()
            && after.starts_with('(')
            && !before.ends_with("::")
        {
            self.cpp_free_call(&a.here, &a.text, word, a.range.start, hits)
        } else {
            hits
        };
        let declared = self.c_member_owners(a, &hits, on);
        if declared.iter().any(Option::is_some) {
            let found = hits
                .into_iter()
                .zip(declared)
                .filter_map(|(hit, owner)| {
                    Some(Candidate {
                        hit,
                        reason: Reason::Path(owner?),
                    })
                })
                .collect();
            let found = self.c_same_overload(a, found);
            self.show_found(a, found);
            return None;
        }
        let (fallback, hits): (Vec<Hit>, Vec<Hit>) = hits.into_iter().partition(|h| {
            self.text_of(&h.path)
                .is_some_and(|t| search::c_fallback(&t, h.line1, word))
        });
        if hits.is_empty() {
            *fallbacks = fallback;
        }
        let parameter = search::c_parameter(&a.text, self.line + 1, word);
        let one = search::c_one_definition(word, &hits, |p| self.text_of(p))
            .filter(|_| !on && !parameter)
            .filter(|o| !self.c_unreached_header(&a.here, &hits[o.hit_index].path));
        Some(match one {
            Some(one) => {
                *aside = Some(one.set_aside_note);
                vec![hits[one.hit_index].clone()]
            }
            None => hits,
        })
    }

    fn c_included_texts(&mut self, here: &std::path::Path) -> Vec<(PathBuf, String)> {
        (self.c_reached(here).into_iter())
            .filter(|f| f.is_relative() && f != here)
            .filter_map(|f| Some((f.clone(), self.text_of(&f)?)))
            .collect()
    }

    fn c_delegated_hit(&self, a: &Asked, file: Option<PathBuf>, line1: usize) -> Hit {
        let (kind, word) = (Kind::C, &a.word);
        let (declared, text) = match file {
            Some(path) => {
                let text = self.text_of(&path).unwrap_or_default();
                let hit = Hit {
                    text: text.lines().nth(line1.saturating_sub(1)).unwrap_or_default().to_owned(),
                    path,
                    line1,
                    byte_col: None,
                    deleted: None,
                };
                (hit, text)
            }
            None => (self.line_hit(&a.here, line1), a.text.clone()),
        };
        let (Some(own), Some(sig)) = (
            search::cpp_member_scope(&text, line1, word),
            search::cpp_declared_signature(&text, line1, word),
        ) else {
            return declared;
        };
        let pattern = format!(r"\b{0}\s*::\s*{0}\s*\(", regex::escape(word));
        let defined: Vec<Hit> = (self.project_definitions(kind, &a.here, word, &pattern))
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    search::cpp_member_scope(&t, h.line1, word).as_ref() == Some(&own)
                        && search::cpp_defined_signature(&t, h.line1, word).as_ref() == Some(&sig)
                })
            })
            .collect();
        match <[Hit; 1]>::try_from(defined) {
            Ok([h]) => h,
            Err(_) => declared,
        }
    }

    fn c_same_overload(&self, a: &Asked, found: Vec<Candidate>) -> Vec<Candidate> {
        self.c_same_overload_of(a, &found).unwrap_or(found)
    }

    pub(super) fn c_same_overload_of(
        &self,
        a: &Asked,
        found: &[Candidate],
    ) -> Option<Vec<Candidate>> {
        let mine = search::cpp_defined_signature(&a.text, self.line + 1, &a.word)?;
        let same: Vec<Candidate> = (found.iter())
            .filter(|c| {
                self.text_of(&c.hit.path)
                    .and_then(|t| search::cpp_declared_signature(&t, c.hit.line1, &a.word))
                    .as_ref()
                    == Some(&mine)
            })
            .cloned()
            .collect();
        let other = |c: &Candidate| c.hit.path != a.here || c.hit.line1 != self.line + 1;
        same.iter().any(other).then_some(same)
    }

    fn c_member_owners(&self, a: &Asked, hits: &[Hit], on: bool) -> Vec<Option<String>> {
        let line = self.line_str();
        let before = a.before.trim_end();
        hits.iter()
            .map(|h| {
                let scopes = self
                    .text_of(&h.path)
                    .filter(|_| on && before.ends_with("::"))
                    .and_then(|t| search::c_member_class(&t, h.line1, &a.word))?;
                search::c_defines_member(line, &scopes, &a.word)
                    .then(|| scopes.last().cloned())
                    .flatten()
            })
            .collect()
    }
}

fn lua_qualified_elsewhere(here: &std::path::Path, word: &str, hits: &[Hit]) -> bool {
    let table = regex::Regex::new(&format!(
        r"^\s*function\s+[\w.:]*[.:]{}\b",
        regex::escape(word)
    ));
    hits.iter()
        .any(|h| h.path != here && table.as_ref().is_ok_and(|re| re.is_match(&h.text)))
}

fn c_namespace(word: &str, hits: Vec<Hit>, after_tag_keyword: bool, called: bool) -> Vec<Hit> {
    static RETURN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"^\s*return\b").expect("the regex is a literal")
    });
    let Ok(tag_decl) = regex::Regex::new(&format!(
        r"^\s*(?:typedef\s+)?(?:struct|union|enum)\s+{}\s*(?:\{{|;|$)",
        regex::escape(word)
    )) else {
        return hits;
    };
    let hits: Vec<Hit> = hits
        .into_iter()
        .filter(|h| !RETURN.is_match(&h.text))
        .collect();
    let tag = |h: &Hit| tag_decl.is_match(&h.text);
    let keep: fn(bool) -> bool = if after_tag_keyword {
        |is_tag| is_tag
    } else if called {
        |is_tag| !is_tag
    } else {
        return hits;
    };
    if hits.iter().any(|h| keep(tag(h))) {
        hits.into_iter().filter(|h| keep(tag(h))).collect()
    } else {
        hits
    }
}
