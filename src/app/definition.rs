mod by_name;
mod early;
mod imports;
mod locals;
mod members;
mod pathed;
mod receivers;

use super::python::{enclosing_def_lines1, starts_by_assigning};
use super::resolution::resolution;
use super::{
    App, Candidate, FileLine, HashMap, Hit, Kind, Path, PathBuf, PickerKind, Reason, Regex, Typed,
    bound, cs_typed, external, import_line, imported, is_word, jvm, names_itself, python, rust,
    search, typed, whole_at, word_col,
};
use early::Asked;
use imports::Imported;
use locals::Near;
use members::Patterns;
use receivers::Walk;

impl App {
    pub(super) fn definition_word(
        &self,
        kind: Option<Kind>,
    ) -> Option<(std::ops::Range<usize>, String)> {
        let (range, word) = self.word_here(kind)?;
        let mut word = word.to_owned();
        let search::LineAsRead {
            line: written,
            word_start: start,
        } = self.written(kind, range.start);
        let before = written.get(..start)?;
        let after = self.line_str().get(range.end..)?.trim_start();
        if kind == Some(Kind::Ruby)
            && before.ends_with('.')
            && !before.ends_with("..")
            && !before.trim_start().starts_with("def ")
            && !word.ends_with(['?', '!'])
            && after
                .strip_prefix('=')
                .is_some_and(|a| !a.starts_with(['=', '~', '>']))
        {
            word.push('=');
        }
        Some((range, word))
    }

    pub(super) fn goto_definition(&mut self) {
        self.anonymous_named.set(None);
        let kind = self.kind();
        if self.answered_without_word(kind) {
            return;
        }
        let Some(mut a) = self.asked(kind) else {
            return;
        };
        if self.answered_early(&mut a) {
            return;
        }
        let Some(near) = self.scope_near(&a) else {
            return;
        };
        if self.answered_by_members(&a, &near) {
            return;
        }
        let Some(p) = self.patterns(&a, &near) else {
            return;
        };
        self.define_by_patterns(a, near, p);
    }

    fn define_by_patterns(&mut self, mut a: Asked, mut near: Near, mut p: Patterns) {
        if self.answered_by_rust_or_name(&a, &near, &p) {
            return;
        }
        let mut walk = Walk {
            broke: None,
            cs_walked: None,
            declares_here: self.declares_here(&a),
        };
        if self.answered_by_go_ts_swift(&a, &near, &p, walk.declares_here)
            || self.answered_by_cs(&a, &near, &p, &mut walk)
            || self.answered_by_typed(&a, &near, &p, &mut walk)
            || self.answered_by_rust_crate(&a, &near)
        {
            return;
        }
        let import = self.import_path(&a, &mut near, &p);
        if self.answered_by_missing_package(&a, import.as_deref()) {
            return;
        }
        let Some(imported) = self.imported(&a, &near, import) else {
            return;
        };
        if self.answered_by_globals(&a, &near, &p) {
            return;
        }
        let Some(q) = self.pathed(&mut a, &near, &mut p) else {
            return;
        };
        if self.answered_by_elixir(&a, &near) || self.answered_by_cs_path(&a, &near, &q) {
            return;
        }
        self.define_by_name(&a, &near, &p, &walk, &imported);
    }

    fn label_targets(
        &mut self,
        kind: Kind,
        here: &Path,
        word: &str,
        line0: usize,
        byte_col: usize,
        owner: &search::Owner,
    ) -> Vec<Candidate> {
        let (cursor_line, cursor_col) = (self.line, self.col);
        let message = std::mem::take(&mut self.message);
        (self.line, self.col) = (line0, byte_col);
        let name = self.definition_word(Some(kind)).map(|(_, w)| w);
        let text = self.buf.lines.join("\n");
        let owners = if let Some(decl) = name
            .as_deref()
            .and_then(|n| search::own_type(kind, &text, line0, n))
        {
            vec![Candidate {
                hit: Hit {
                    deleted: None,
                    path: here.to_path_buf(),
                    line1: decl,
                    byte_col: None,
                    text: self.buf.lines[decl - 1].clone(),
                },
                reason: Reason::File,
            }]
        } else {
            self.probe = Some(Vec::new());
            self.goto_definition();
            self.probe.take().unwrap_or_default()
        };
        (self.line, self.col, self.message) = (cursor_line, cursor_col, message);
        let reads_outside = kind == Kind::Rust && *owner == search::Owner::Typed;
        let offer = std::mem::take(&mut self.offer_only)
            || (!reads_outside && owners.iter().any(|c| c.hit.path.is_absolute()))
            || (owners.iter().all(|c| !c.reason.proven())
                && name
                    .as_deref()
                    .is_some_and(|n| self.receiver_outside(kind, here, line0, byte_col, n)));
        self.truncated.set(false);
        let Some(name) = name else {
            return Vec::new();
        };
        let what = match owner {
            search::Owner::Args => "parameter",
            search::Owner::Object(_) | search::Owner::Typed => "field",
        };
        let mut out: Vec<Candidate> = Vec::new();
        for c in owners {
            if (c.hit.path.is_absolute() && !reads_outside)
                || (c.hit.path == here && c.hit.line1 == cursor_line + 1)
            {
                continue;
            }
            let Some(at) = self.owner_label_lines(kind, here, word, owner, &name, &c.hit) else {
                continue;
            };
            let by_name = if c.reason.proven() {
                ""
            } else {
                ", found by name"
            };
            for (path, mut line, of) in at {
                if kind == Kind::Swift && *owner == search::Owner::Args {
                    line = self
                        .text_of(&path)
                        .map_or(line, |t| search::swift_signature_line(&t, line));
                }
                if out
                    .iter()
                    .any(|o| o.hit.path == path && o.hit.line1 == line)
                {
                    continue;
                }
                let text = self
                    .text_of(&path)
                    .and_then(|t| t.lines().nth(line - 1).map(str::to_owned))
                    .unwrap_or_default();
                out.push(Candidate {
                    hit: Hit {
                        deleted: None,
                        path,
                        line1: line,
                        byte_col: None,
                        text,
                    },
                    reason: Reason::Label(format!("{what} of {of}{by_name}")),
                });
            }
        }
        self.offer_only = offer && !out.is_empty();
        out
    }

    fn owner_label_lines(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        owner: &search::Owner,
        name: &str,
        c: &Hit,
    ) -> Option<Vec<(PathBuf, usize, String)>> {
        let text = self.text_of(&c.path)?;
        let of = search::qualified(kind, &text, c.line1, name).unwrap_or_else(|| name.to_owned());
        let mut at: Vec<(PathBuf, usize, String)> =
            search::label_lines(kind, &text, c.line1, name, word, owner)
                .into_iter()
                .map(|l| (c.path.clone(), l, of.clone()))
                .collect();
        if at.is_empty()
            && let search::Owner::Object(n) = owner
            && let Some(ty) = search::object_type(kind, &text, c.line1, name, *n)
        {
            let cut = self.truncated.get();
            let pattern = search::def_patterns(kind, &ty).join("|");
            let decls = self.project_definitions(kind, here, &ty, &pattern);
            self.truncated.set(cut);
            if let [d] = decls.as_slice()
                && let Some(t) = self.text_of(&d.path)
            {
                at = search::label_lines(kind, &t, d.line1, &ty, word, &search::Owner::Typed)
                    .into_iter()
                    .map(|l| (d.path.clone(), l, ty.clone()))
                    .collect();
            }
        }
        Some(at)
    }

    fn receiver_outside(
        &self,
        kind: Kind,
        here: &Path,
        line0: usize,
        byte_col: usize,
        name: &str,
    ) -> bool {
        let lines = &self.buf.lines;
        let Some(head) = lines[line0]
            .get(..byte_col + 1)
            .and_then(|h| h.strip_suffix(name))
            .and_then(|h| h.strip_suffix('.'))
        else {
            return false;
        };
        let recv = head
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next()
            .unwrap_or_default();
        let r = regex::escape(recv);
        let Ok(re) = Regex::new(&format!(
            r"\b{r}\s*:\s*([A-Z][\w.]*)|\b([A-Z][\w.]*)(?:<[^>]*>)?\s+{r}\b"
        )) else {
            return false;
        };
        let ty = lines[..=line0].iter().rev().find_map(|l| {
            let c = re.captures(l)?;
            let t = c.get(1).or_else(|| c.get(2))?.as_str();
            t.rsplit('.').next().map(str::to_owned)
        });
        let Some(ty) = ty.filter(|_| !recv.is_empty()) else {
            return false;
        };
        let cut = self.truncated.get();
        let pattern = search::def_patterns(kind, &ty).join("|");
        let none = self
            .project_definitions(kind, here, &ty, &pattern)
            .is_empty();
        self.truncated.set(cut);
        none
    }

    fn literal_field(
        &self,
        kind: Kind,
        file: &Path,
        written: &str,
        word: &str,
    ) -> Result<Option<Vec<Candidate>>, GoTypeUnread> {
        let Some(ty) = self.struct_decl(kind, file, written, 0)? else {
            return Ok(None);
        };
        let label = format!(
            "{}{{\u{2026}}}",
            written.split('[').next().unwrap_or(written)
        );
        Ok(Some(
            self.field_of(kind, &ty, word, false)
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::Receiver(label.clone()),
                })
                .collect(),
        ))
    }

    pub(super) fn struct_decl(
        &self,
        kind: Kind,
        file: &Path,
        written: &str,
        depth: usize,
    ) -> Result<Option<Typed>, GoTypeUnread> {
        static DEFINED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^(?:type\s+)?[A-Za-z_]\w*(?:\[[^\]]*\])?\s+([^/{]*)")
                .expect("the regex is a literal")
        });
        let ty = self.type_decl(kind, file, written).ok_or(GoTypeUnread)?;
        let text = self.text_of(&ty.path).ok_or(GoTypeUnread)?;
        let line = text.lines().nth(ty.line - 1).ok_or(GoTypeUnread)?;
        let over = DEFINED.captures(line.trim_start()).ok_or(GoTypeUnread)?[1]
            .trim()
            .to_owned();
        if over.starts_with("struct") {
            return Ok(Some(ty));
        }
        let named = search::type_path(kind, &over).is_some() && !over.starts_with('*');
        if named && depth < 8 {
            self.struct_decl(kind, &ty.path, &over, depth + 1)
        } else {
            Ok(None)
        }
    }

    fn go_bare(&mut self, here: &Path, word: &str, imports: &[search::Import]) -> Vec<Candidate> {
        let kind = Kind::Go;
        let by = |hits: Vec<Hit>, reason: Reason| -> Vec<Candidate> {
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect()
        };
        let own = self.package_declarations(kind, here, word, None);
        let own = self.host_built(kind, own);
        if !own.is_empty() {
            return by(own, Reason::ByName);
        }
        let dot = [".".to_owned()];
        for search::Import { path, .. } in imports.iter().filter(|i| i.name == ".") {
            let found = if let Some(found) = self.imported_definitions(kind, here, word, &dot, path)
            {
                found
            } else {
                let only = [search::Import {
                    name: ".".to_owned(),
                    path: path.clone(),
                }];
                self.external_definitions(kind, word, &dot, false, &only, true)
                    .unwrap_or_default()
            };
            if !found.is_empty() {
                return found;
            }
        }
        self.external_files(kind);
        let builtin: Vec<PathBuf> = self
            .external
            .get(&kind)
            .and_then(|(roots, _)| roots.first())
            .map(|goroot| goroot.join("builtin/builtin.go"))
            .filter(|f| f.is_file())
            .into_iter()
            .collect();
        let pattern = search::def_patterns(kind, word).join("|");
        let hits = self.external_grep(kind, &builtin, &pattern);
        by(
            self.declaring(kind, word, hits),
            Reason::Path("builtin".to_owned()),
        )
    }

    fn names_module(&self, first: &str) -> bool {
        matches!(first, "crate" | "self" | "super")
            || self.files.iter().any(|f| {
                f.file_stem().is_some_and(|s| s == first)
                    || f.parent().is_some_and(|d| d.ends_with(first))
            })
    }

    pub(super) fn show_definitions(
        &mut self,
        kind: Kind,
        word: &str,
        here: &Path,
        mut found: Vec<Candidate>,
        broke: Option<&str>,
    ) {
        let anonymous = self.anonymous_named.take();
        self.keep_shown(kind, word, &mut found);
        let superclass = kind == Kind::Ruby
            && search::definition_word(Some(kind), self.line_str(), self.col).is_some_and(
                |(r, _)| {
                    Regex::new(r"^\s*class\s+[\w:]+\s*<\s*[\w:]*$").is_ok_and(|re| {
                        self.line_str()
                            .get(..r.start)
                            .is_some_and(|b| re.is_match(b))
                    })
                },
            );
        if superclass {
            found.retain(|c| c.hit.line1 != self.line + 1 || c.hit.path != here);
        }
        if kind == Kind::C {
            self.c_rows(word, here, &mut found);
        }
        let as_type = kind == Kind::Go
            && search::definition_word(Some(kind), self.line_str(), self.col).is_some_and(
                |(r, w)| w == word && search::go_param_type(self.line_str(), word, r.start),
            )
            && found
                .iter()
                .any(|c| search::declares_type(kind, &c.hit.text));
        if as_type {
            found.retain(|c| search::declares_type(kind, &c.hit.text));
        }
        if kind == Kind::Python {
            self.python_implementation(word, here, &mut found);
        }
        let all = found.len();
        let own = (found.iter())
            .find(|c| c.hit.line1 == self.line + 1 && c.hit.path == here)
            .map(|c| c.hit.clone());
        if all > 1 {
            found.retain(|c| c.hit.line1 != self.line + 1 || c.hit.path != here);
        }
        let offer_only = std::mem::take(&mut self.offer_only);
        let truncated = self.truncated.take();
        let on_member = || {
            let at = search::definition_word(Some(kind), self.line_str(), self.col);
            let bare = at.is_some_and(|(r, w)| {
                w == word
                    && self
                        .line_str()
                        .get(..r.start)
                        .is_some_and(|b| !b.ends_with('.'))
            });
            bare && search::member_or_signature(kind, word)
                .and_then(|p| Regex::new(&p.join("|")).ok())
                .is_some_and(|re| re.is_match(self.line_str()))
                && search::owner_decl(kind, &self.buf.lines.join("\n"), self.line + 1).is_some()
        };
        let namesakes = !as_type
            && found.iter().all(|c| !c.reason.proven())
            && !found.is_empty()
            && (found.len() < all || on_member())
            && found
                .iter()
                .any(|c| c.hit.line1 != self.line + 1 || c.hit.path != here)
            && self.on_declared_name(kind, word, false);
        let counterpart =
            namesakes && kind == Kind::C && all == 2 && self.c_counterpart(word, own, &found);
        let namesakes = namesakes && !counterpart;
        let word_chars = search::word_chars(Some(kind), true);
        let form = |t: &str| {
            t.get(..word_col(t, word, word_chars))
                .map(|h| h.trim().to_owned())
        };
        let copy = found.len() < all
            && !counterpart
            && !namesakes
            && search::definition_word(Some(kind), self.line_str(), self.col).is_some_and(
                |(r, _)| {
                    self.line_str()
                        .get(..r.start)
                        .is_some_and(|b| !b.ends_with('.'))
                },
            )
            && matches!(found.as_slice(), [c] if !c.reason.proven() && form(&c.hit.text).is_some_and(|f| Some(f) == form(self.line_str())));
        found.sort_by_cached_key(|c| search::rank(&c.hit.path, Some(here), true).tier);
        found.truncate(search::MAX_HITS);
        if let Some(probe) = &mut self.probe {
            *probe = found;
            self.offer_only = offer_only || broke.is_some();
            return;
        }
        match found.as_slice() {
            [] => self.message = resolution(word, None, &found, broke, truncated),
            [_] if !namesakes && !offer_only && !truncated && !copy => {
                self.jump_to_only(kind, word, &found, broke, anonymous);
            }
            _ => self.pick_definitions(kind, word, found, broke, namesakes, truncated),
        }
    }

    fn c_counterpart(&self, word: &str, own: Option<Hit>, found: &[Candidate]) -> bool {
        let (Some(own), [other]) = (own, found) else {
            return false;
        };
        let owner = |h: &Hit| {
            self.text_of(&h.path)
                .and_then(|t| search::cpp_owner(&t, h.line1, word))
        };
        let pair = [own, other.hit.clone()];
        owner(&pair[0]).is_some_and(|o| owner(&pair[1]) == Some(o))
            && search::c_one_definition(word, &pair, |p| self.text_of(p)).is_some()
    }

    fn keep_shown(&self, kind: Kind, word: &str, found: &mut Vec<Candidate>) {
        if kind == Kind::TsJs {
            let head = search::ts_method_head(word);
            found.retain(|c| {
                !search::ts_call_statement(&head, &c.hit.text, c.hit.line1, || {
                    self.hit_text(&c.hit)
                })
            });
            let called = search::definition_word(Some(kind), self.line_str(), self.col)
                .is_some_and(|(r, w)| {
                    w == word
                        && self
                            .line_str()
                            .get(r.end..)
                            .is_some_and(|t| t.starts_with('('))
                });
            let namespace = Regex::new(&format!(
                r"^\s*(?:export\s+)?(?:declare\s+)?namespace\s+{}\b",
                regex::escape(word)
            ))
            .expect("an escaped name keeps the pattern valid");
            if called && found.iter().any(|c| !namespace.is_match(&c.hit.text)) {
                found.retain(|c| !namespace.is_match(&c.hit.text));
            }
            if called {
                self.ts_overloads(word, found);
            }
        }
        if found.len() <= 500 {
            let mut literal: HashMap<(PathBuf, bool), Vec<bool>> = HashMap::new();
            found.retain(|c| {
                let lines = (literal.entry((c.hit.path.clone(), c.hit.deleted.is_some())))
                    .or_insert_with(|| self.hidden_now(kind, &c.hit));
                let call = kind == Kind::TsJs
                    && c.hit.text.trim_end().ends_with('<')
                    && !search::declares_type(kind, &c.hit.text)
                    && !Regex::new(r"\b(?:extends|implements)\b")
                        .is_ok_and(|re| re.is_match(&c.hit.text))
                    && self
                        .hit_text(&c.hit)
                        .is_some_and(|t| !search::declares_wrapped_generic(&t, c.hit.line1));
                let literal = lines.get(c.hit.line1 - 1).copied().unwrap_or(false)
                    && !self.doc_tag(kind, &c.hit)
                    && !(c.reason == Reason::Local && search::component(&c.hit.path));
                !call && !literal
            });
        }
    }

    fn ts_overloads(&self, word: &str, found: &mut Vec<Candidate>) {
        let on_one = |c: &Candidate| c.hit.line1 == self.line + 1 && c.hit.text == self.line_str();
        if found.len() < 2
            || found
                .iter()
                .any(|c| c.hit.path != found[0].hit.path || on_one(c))
        {
            return;
        }
        let Some(text) = self.hit_text(&found[0].hit) else {
            return;
        };
        if !found.is_sorted_by_key(|c| c.hit.line1) {
            return;
        }
        let sigs: Option<Vec<search::TsSignature>> = found
            .iter()
            .map(|c| search::ts_signature(&text, c.hit.line1, word))
            .collect();
        let Some(sigs) = sigs else {
            return;
        };
        let lines: Vec<&str> = text.lines().collect();
        let between = |s: &search::TsSignature, next: usize| {
            lines.get(s.end_line1..next - 1).is_some_and(|gap| {
                gap.iter().all(|l| {
                    let t = l.trim();
                    t.is_empty() || t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
                })
            })
        };
        let Some((last, signatures)) = sigs.split_last() else {
            return;
        };
        let one_set = last.body
            && signatures.iter().all(|s| !s.body)
            && sigs
                .iter()
                .zip(&found[1..])
                .all(|(s, next)| between(s, next.hit.line1));
        if !one_set {
            return;
        }
        found.pop();
        let arity = search::definition_word(Some(Kind::TsJs), self.line_str(), self.col)
            .and_then(|(r, _)| search::ts_call_arity(&self.buf.lines, self.line, r.end));
        let Some(n) = arity else {
            return;
        };
        let fits = |s: &search::TsSignature| n >= s.fewest && s.most.is_none_or(|m| n <= m);
        if signatures.iter().any(fits) {
            let mut fit = signatures.iter().map(fits);
            found.retain(|_| fit.next().unwrap_or(false));
        }
    }

    fn jump_to_only(
        &mut self,
        kind: Kind,
        word: &str,
        found: &[Candidate],
        broke: Option<&str>,
        anonymous: Option<Typed>,
    ) {
        let one = &found[0];
        let word_chars = search::word_chars(Some(kind), true);
        let path = self.root.join(&one.hit.path);
        let name = jvm::declared_as(kind, word, &one.hit.text);
        let target = self
            .hit_text(&one.hit)
            .filter(|_| !matches!(one.reason, Reason::Module(_) | Reason::Label(_)))
            .and_then(|text| {
                let named = anonymous.filter(|ty| {
                    ty.path == one.hit.path
                        && search::owner_decl(kind, &text, one.hit.line1) == Some(ty.line)
                });
                match named {
                    Some(ty) => Some(format!("{}.{name}", ty.name)),
                    None => search::qualified(kind, &text, one.hit.line1, &name),
                }
            });
        let status = resolution(word, target.as_deref(), found, broke, false);
        let col = word_col(&one.hit.text, &name, word_chars);
        match one.hit.deleted {
            Some(_) => self.jump_to_deleted(&path, one.hit.line1, col),
            None => self.jump_to_col(&path, one.hit.line1, col),
        }
        if self.buf.path.as_deref() == Some(path.as_path()) {
            self.message = status;
        }
    }

    fn pick_definitions(
        &mut self,
        kind: Kind,
        word: &str,
        found: Vec<Candidate>,
        broke: Option<&str>,
        namesakes: bool,
        truncated: bool,
    ) {
        let status = if namesakes {
            let others = if found.len() == 1 { "other" } else { "others" };
            format!("{word}: at a declaration, {} {others} by name", found.len())
        } else {
            resolution(word, None, &found, broke, truncated)
        };
        let items = self.definition_items(kind, word, found);
        self.show_picker(PickerKind::Definitions, items);
        if let Some(p) = &mut self.picker {
            p.title = p
                .title
                .replacen(PickerKind::Definitions.title(), &status, 1);
        }
        self.message = status;
    }

    pub(super) fn project_definitions(
        &self,
        kind: Kind,
        here: &Path,
        word: &str,
        pattern: &str,
    ) -> Vec<Hit> {
        let hits = self.project_grep(kind, here, pattern);
        let mut hits = self.declaring(kind, word, hits);
        if kind == Kind::TsJs {
            let mut lines: HashMap<PathBuf, Vec<String>> = HashMap::new();
            hits.retain(|h| {
                h.path == here
                    || !search::ts_nested_local(
                        lines.entry(h.path.clone()).or_insert_with(|| {
                            self.text_of(&h.path)
                                .map_or_else(Vec::new, |t| t.lines().map(str::to_owned).collect())
                        }),
                        h.line1,
                    )
            });
        }
        if kind == Kind::Make {
            let mut recipe = self.make_recipe_rule(Some(here), word);
            hits.retain(|h| recipe(h).unwrap_or(true));
            if hits.is_empty() {
                let fallback = search::make_fallback_patterns(word).join("|");
                hits = self
                    .grep(&fallback, false, false, |p| {
                        search::in_def_scope(kind, here, p)
                    })
                    .unwrap_or_default();
                self.note_cut(&hits);
                hits.retain(|h| recipe(h).is_none());
            }
        }
        if kind == Kind::Swift {
            self.swift_unseen(here, &mut hits);
        }
        if kind == Kind::Shell {
            let lines: Vec<&str> = self.buf.lines.iter().map(String::as_str).collect();
            let mine = search::shell_function_at(&lines, self.line);
            hits.retain(|h| {
                if let Some(binds) = search::shell_loop_binds(&h.text, word) {
                    return binds && h.path == here;
                }
                let Some(text) = self.text_of(&h.path) else {
                    return true;
                };
                let lines: Vec<&str> = text.lines().collect();
                let at = h.line1 - 1;
                match lines.get(at).and_then(|l| search::shell_local_of(l)) {
                    Some(_) => search::shell_function_at(&lines, at)
                        .is_none_or(|f| h.path == here && Some(f) == mine),
                    None => true,
                }
            });
            let lead = search::definition_word(Some(kind), self.line_str(), self.col)
                .and_then(|(r, _)| self.line_str().get(..r.start));
            if let Some(command) = lead.and_then(search::shell_command_position) {
                let sorted: Vec<Hit> = hits
                    .iter()
                    .filter(|h| search::shell_names_a_command(&h.text) == command)
                    .cloned()
                    .collect();
                if !sorted.is_empty() {
                    hits = sorted;
                }
            }
        }
        if kind == Kind::Sql {
            self.sql_hits(here, word, &mut hits);
        }
        hits
    }

    fn sql_hits(&self, here: &Path, word: &str, hits: &mut Vec<Hit>) {
        if let Some((range, _)) = self.word_here(Some(Kind::Sql)).filter(|(_, w)| *w == word)
            && let Some(lead) = self.line_str().get(..range.start)
            && hits
                .iter()
                .any(|h| search::sql_same_schema(lead, &h.text) == Some(true))
        {
            hits.retain(|h| search::sql_same_schema(lead, &h.text) == Some(true));
        }
        if !hits
            .iter()
            .any(|h| h.path == here && h.line1 == self.line + 1)
        {
            let cte = Regex::new(&search::sql_cte(word)).expect("an escaped name keeps it valid");
            let text = self.text_of(here).unwrap_or_default();
            let sees = |h: &Hit| {
                if h.path == here {
                    search::sql_cte_sees(&text, h.line1, word, self.line + 1, self.col)
                } else {
                    Some(false)
                }
            };
            let known = hits
                .iter()
                .filter(|h| cte.is_match(&h.text))
                .all(|h| sees(h).is_some());
            let seen = hits.iter().any(|h| sees(h) == Some(true));
            hits.retain(|h| {
                if cte.is_match(&h.text) {
                    !known || sees(h) == Some(true)
                } else {
                    !known || !seen
                }
            });
        }
        if hits.iter().any(|h| !search::sql_renames(&h.text)) {
            hits.retain(|h| !search::sql_renames(&h.text));
        }
    }
}

pub(super) struct GoTypeUnread;
