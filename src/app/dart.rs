use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, Regex, search};

impl App {
    pub(super) fn word_here(&self, kind: Option<Kind>) -> Option<(std::ops::Range<usize>, &str)> {
        let line = self.line_str();
        let literal = || {
            search::literal_lines(Kind::Dart, &self.buf.lines.join("\n")).get(self.line)
                == Some(&true)
        };
        if kind == Some(Kind::Dart) && literal() {
            search::word_at(line, self.col, "")
        } else {
            search::definition_word(kind, line, self.col)
        }
    }

    pub(super) fn dart_imported(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        if let Some(found) = self.dart_member(here, text, before, chain, word) {
            return Some(found);
        }
        if let Some(found) = self.dart_own_member(here, text, before, chain, word) {
            return Some(found);
        }
        let qualifier = self
            .word_here(Some(Kind::Dart))
            .and_then(|(range, _)| self.line_str().get(range.end..))
            .is_some_and(|r| r.starts_with('.') && !r.starts_with(".."));
        let member = || {
            let declares = Regex::new(&search::def_patterns(Kind::Dart, word).join("|")).ok();
            search::dart_body_at(text, self.line).is_some_and(|(body, _)| {
                text.lines().enumerate().any(|(i, l)| {
                    body.contains(&(i + 1)) && declares.as_ref().is_some_and(|d| d.is_match(l))
                })
            })
        };
        if chain.is_empty()
            && qualifier
            && !before.trim_end().ends_with('.')
            && search::bindings(Kind::Dart, text, self.line + 1, word).is_empty()
            && !member()
            && let Some(d) = search::dart_directives(text)
                .into_iter()
                .flatten()
                .find(|d| d.keyword == "import" && d.prefix.as_deref() == Some(word))
        {
            let hit = Hit {
                deleted: None,
                path: here.to_path_buf(),
                line1: d.line1,
                byte_col: None,
                text: self.buf.lines.get(d.line1 - 1)?.clone(),
            };
            let reason = Reason::Import(d.uri);
            return Some(vec![Candidate { hit, reason }]);
        }
        let imports = search::dart_imports(text);
        let bound = |name: &str, prefix: bool| {
            imports
                .iter()
                .find(|i| i.name == name && i.is_prefix == prefix)
                .map(|i| i.uri_as_written.clone())
        };
        let (uri, within) = match chain {
            [] if before.trim_end().ends_with('.') => return None,
            [] => (bound(word, false)?, None),
            [p] => match bound(p, true) {
                Some(uri) => (uri, None),
                None => (bound(p, false)?, Some(format!("{p}.{word}"))),
            },
            [p, class] => (bound(p, true)?, Some(format!("{class}.{word}"))),
            _ => return None,
        };
        let sdk = search::dart_sdk();
        let file = search::dart_uri_file(&self.root, here, &uri, sdk.as_deref())?;
        let bare = match chain {
            [_, class] => Some(search::dart_bare_constructor(class, word)),
            _ => None,
        };
        let built = chain.is_empty().then(|| format!("{word}.{word}"));
        let found =
            self.dart_declared_in(&file, word, bare, &[within, built], &Reason::Import(uri));
        (!found.is_empty()).then_some(found)
    }

    fn dart_declared_in(
        &self,
        file: &Path,
        word: &str,
        bare: Option<String>,
        wanted: &[Option<String>],
        reason: &Reason,
    ) -> Vec<Candidate> {
        let mut patterns = search::def_patterns(Kind::Dart, word);
        self.spelling_cut(Kind::Dart, word, &mut patterns);
        patterns.extend(bare);
        let pattern = patterns.join("|");
        let hits = if file.is_absolute() {
            self.external_grep(Kind::Dart, &[file.to_path_buf()], &pattern)
        } else {
            self.grep(&pattern, false, false, |p| p == file)
                .unwrap_or_default()
        };
        let Some(source) = self.text_of(file) else {
            return Vec::new();
        };
        let literal = search::literal_lines(Kind::Dart, &source);
        self.declaring(Kind::Dart, word, hits)
            .into_iter()
            .filter(|h| {
                literal.get(h.line1 - 1) != Some(&true)
                    && wanted.contains(&search::qualified(Kind::Dart, &source, h.line1, word))
            })
            .map(|hit| Candidate {
                hit,
                reason: reason.clone(),
            })
            .collect()
    }

    pub(super) fn dart_reachable(&self, here: &Path, member: bool, found: &mut Vec<Candidate>) {
        let package = |dir: &str, p: &Path| {
            p.ancestors().any(|a| {
                a.file_name().is_some_and(|n| n == dir)
                    && a.parent()
                        .is_some_and(|d| self.root.join(d).join("pubspec.yaml").is_file())
            })
        };
        let in_lib = package("lib", here);
        found.retain(|c| {
            let tested = in_lib && !c.hit.path.is_absolute() && package("test", &c.hit.path);
            let local = || {
                self.text_of(&c.hit.path)
                    .is_some_and(|t| search::dart_in_function(&t, c.hit.line1))
            };
            !(tested || (member && local()))
        });
    }

    pub(super) fn dart_visible(
        &self,
        here: &Path,
        word: &str,
        tail: &str,
        found: &mut Vec<Candidate>,
    ) {
        if tail.trim_start().starts_with(':') {
            return;
        }
        let Some(files) = self.dart_visible_files(here, word, None) else {
            return;
        };
        let top = |c: &Candidate| !c.hit.text.starts_with(char::is_whitespace);
        if !found.iter().any(|c| top(c) && files.contains(&c.hit.path)) {
            return;
        }
        let Some(text) = self.text_of(here) else {
            return;
        };
        let body = search::dart_body_at(&text, self.line);
        let built = format!("{word}.{word}");
        found.retain(|c| {
            let constructor = || {
                self.text_of(&c.hit.path).is_some_and(|t| {
                    search::qualified(Kind::Dart, &t, c.hit.line1, word).as_ref() == Some(&built)
                })
            };
            if c.hit.path.is_absolute() {
                true
            } else if top(c) {
                files.contains(&c.hit.path)
            } else if constructor() {
                search::dart_called(tail) && files.contains(&c.hit.path)
            } else {
                body.as_ref().is_none_or(|(lines, supers)| {
                    *supers || (c.hit.path == here && lines.contains(&c.hit.line1))
                })
            }
        });
    }

    pub(super) fn dart_unimported(&self, here: &Path, word: &str, found: &mut Vec<Candidate>) {
        let Some((roots, _)) = self.external.get(&Kind::Dart) else {
            return;
        };
        let built = Some(format!("{word}.{word}"));
        let named = |c: &Candidate| {
            c.hit.path.is_absolute()
                && self.text_of(&c.hit.path).is_some_and(|t| {
                    let q = search::qualified(Kind::Dart, &t, c.hit.line1, word);
                    (q.is_none() && !c.hit.text.starts_with(char::is_whitespace)) || q == built
                })
        };
        let homes: Vec<Option<PathBuf>> = found
            .iter()
            .map(|c| search::dart_root_of(roots, &c.hit.path).filter(|_| named(c)))
            .collect();
        let mut wanted: Vec<PathBuf> = homes.iter().flatten().cloned().collect();
        wanted.sort();
        wanted.dedup();
        let Some(reached) = self.dart_import_roots(here, roots, &wanted) else {
            return;
        };
        let unreached: Vec<bool> = homes
            .iter()
            .map(|h| h.as_ref().is_some_and(|r| !reached.contains(r)))
            .collect();
        if unreached.iter().all(|u| *u) {
            return;
        }
        let mut drop = unreached.into_iter();
        found.retain(|_| !drop.next().unwrap_or(false));
    }

    fn dart_import_roots(
        &self,
        here: &Path,
        roots: &[PathBuf],
        wanted: &[PathBuf],
    ) -> Option<Vec<PathBuf>> {
        let sdk = self.dart_sdk();
        let sdk = sdk.as_deref();
        let mut reached: Vec<PathBuf> = roots
            .iter()
            .filter(|r| r.join("core/core.dart").is_file())
            .cloned()
            .collect();
        if wanted.iter().all(|w| reached.contains(w)) {
            return None;
        }
        let (library, directives) = self.dart_library(here.to_path_buf(), sdk, false)?;
        let mut queue: Vec<(PathBuf, search::DartDirective)> = directives
            .into_iter()
            .filter(|d| d.keyword == "import")
            .map(|d| (library[0].clone(), d))
            .collect();
        let mut seen = std::collections::HashSet::new();
        while let Some((from, d)) = queue.pop() {
            for uri in std::iter::once(&d.uri).chain(&d.alternatives) {
                if uri.starts_with("dart:") && uri != "dart:ui" {
                    continue;
                }
                let base = if uri.starts_with("package:") {
                    here
                } else {
                    &from
                };
                let file = search::dart_uri_file(&self.root, base, uri, sdk)?;
                if !seen.insert(file.clone()) {
                    continue;
                }
                if let Some(r) = search::dart_root_of(roots, &file)
                    && !reached.contains(&r)
                {
                    reached.push(r);
                    if wanted.iter().all(|w| reached.contains(w)) {
                        return None;
                    }
                }
                let (library, directives) = self.dart_library(file, sdk, false)?;
                queue.extend(
                    directives
                        .into_iter()
                        .filter(|d| d.keyword == "export")
                        .map(|d| (library[0].clone(), d)),
                );
            }
        }
        Some(reached)
    }

    pub(super) fn dart_outside_constructor(
        &self,
        word: &str,
        tail: &str,
        found: &mut Vec<Candidate>,
    ) {
        let built = format!("{word}.{word}");
        let constructor = |c: &Candidate| {
            self.text_of(&c.hit.path).is_some_and(|t| {
                search::qualified(Kind::Dart, &t, c.hit.line1, word).as_ref() == Some(&built)
            })
        };
        let top = |c: &Candidate| !c.hit.text.starts_with(char::is_whitespace);
        let paired = |c: &Candidate| {
            let mut file = found.iter().filter(|o| o.hit.path == c.hit.path);
            let (Some(a), Some(b), None) = (file.next(), file.next(), file.next()) else {
                return false;
            };
            let (class, made) = if top(a) { (a, b) } else { (b, a) };
            top(class) && !top(made) && constructor(made)
        };
        if found.is_empty() || !found.iter().all(|c| c.hit.path.is_absolute() && paired(c)) {
            return;
        }
        let called = search::dart_called(tail);
        found.retain(|c| top(c) != called);
    }

    fn dart_library(
        &self,
        file: PathBuf,
        sdk: Option<&Path>,
        owned: bool,
    ) -> Option<(Vec<PathBuf>, Vec<search::DartDirective>)> {
        let directives = search::dart_directives(&self.text_of(&file)?)?;
        if let Some(d) = directives.iter().find(|d| d.keyword == "part of") {
            let owner = search::dart_uri_file(&self.root, &file, &d.uri, sdk)?;
            return if owned {
                None
            } else {
                self.dart_library(owner, sdk, true)
            };
        }
        let mut files: Vec<PathBuf> = directives
            .iter()
            .filter(|d| d.keyword == "part")
            .filter_map(|d| search::dart_uri_file(&self.root, &file, &d.uri, sdk))
            .collect();
        files.insert(0, file);
        Some((files, directives))
    }

    fn dart_visible_files(
        &self,
        here: &Path,
        word: &str,
        prefix: Option<&str>,
    ) -> Option<Vec<PathBuf>> {
        let sdk = self.dart_sdk();
        let sdk = sdk.as_deref();
        let (library, directives) = self.dart_library(here.to_path_buf(), sdk, false)?;
        let from = library[0].clone();
        let mut files = if prefix.is_none() {
            library
        } else {
            Vec::new()
        };
        let mut queue: Vec<(PathBuf, search::DartDirective)> = directives
            .into_iter()
            .filter(|d| d.keyword == "import" && d.prefix.as_deref() == prefix)
            .map(|d| (from.clone(), d))
            .collect();
        while let Some((from, d)) = queue.pop() {
            if d.uri.starts_with("dart:")
                || d.show
                    .as_ref()
                    .is_some_and(|s| !s.iter().any(|n| n == word))
                || d.hide.iter().any(|n| n == word)
            {
                continue;
            }
            let file = search::dart_uri_file(&self.root, &from, &d.uri, sdk)?;
            if files.contains(&file) {
                continue;
            }
            if file.is_absolute() {
                files.push(file);
                continue;
            }
            let (library, directives) = self.dart_library(file, sdk, false)?;
            queue.extend(
                directives
                    .into_iter()
                    .filter(|d| d.keyword == "export")
                    .map(|d| (library[0].clone(), d)),
            );
            files.extend(library);
        }
        Some(files)
    }

    fn dart_member(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        if let [r] = chain
            && r == "super"
            && let Some(found) = self.dart_super_param(here, text, word)
        {
            return Some(vec![found]);
        }
        let mut scope = here.to_path_buf();
        let ty = match chain {
            [] if before.ends_with("..") => match search::dart_cascade(before)? {
                Ok(ty) => ty,
                Err(name) => {
                    let (ty, s) = self.dart_bound(here, text, &name, 0)?;
                    scope = s;
                    ty
                }
            },
            [] if search::dart_list_literal(before) => search::DartType {
                prefix: None,
                class: "List".to_owned(),
                constructor: None,
            },
            [] => {
                let (ty, s) = self.dart_chain_type(here, text, before, 0)?;
                scope = s;
                ty
            }
            [r] if r == "this" || r == "super" => {
                search::dart_this_super(text, self.line, r == "super")?
            }
            [_, _, ..] => {
                let (ty, s) = self.dart_links_type(here, text, chain, false, true, 0)?;
                scope = s;
                ty
            }
            [r] => match self.dart_bound(here, text, r, 0) {
                Some((ty, s)) => {
                    scope = s;
                    ty
                }
                None if r.trim_start_matches('_').starts_with(char::is_uppercase)
                    && search::bindings(Kind::Dart, text, self.line + 1, r).is_empty() =>
                {
                    let ty = search::DartType {
                        prefix: None,
                        class: r.clone(),
                        constructor: None,
                    };
                    let file = self.dart_class_file(here, &ty)?;
                    let found = self.dart_member_of(&file, r, word);
                    return (file.is_absolute() && !found.is_empty()).then_some(found);
                }
                None => {
                    let (ty, s) = self.dart_field_type(here, text, r)?;
                    scope = s;
                    ty
                }
            },
        };
        let Some(file) = self.dart_class_file(&scope, &ty) else {
            return self.dart_core_unreached(&scope, &ty, word).then(Vec::new);
        };
        let class = &ty.class;
        if let Some(c) = &ty.constructor
            && !self
                .dart_member_of(&file, class, c)
                .iter()
                .any(|h| h.hit.text.contains(&format!("{class}.{c}")))
        {
            return None;
        }
        let mut found = self.dart_member_of(&file, class, word);
        if found.is_empty() {
            found = self.dart_inherited(&file, class, word, 0);
        }
        self.dart_accessor_side(before, word, &mut found);
        (!found.is_empty()).then_some(found)
    }

    fn dart_super_param(&self, here: &Path, text: &str, word: &str) -> Option<Candidate> {
        let (range, _) = self.word_here(Some(Kind::Dart))?;
        let at = line_offset(text, self.line + 1) + range.start;
        let (ty, named) = search::dart_super_param(text, self.line, at)?;
        let file = self.dart_class_file(here, &ty)?;
        let source = self.text_of(&file)?;
        let line1 = search::dart_constructor_param(&source, &ty.class, named.as_deref(), word)?;
        let hit = Hit {
            deleted: None,
            path: file,
            line1,
            byte_col: None,
            text: source.lines().nth(line1 - 1)?.to_owned(),
        };
        let reason = Reason::Receiver(ty.class);
        Some(Candidate { hit, reason })
    }

    fn dart_bound(
        &self,
        here: &Path,
        text: &str,
        name: &str,
        depth: usize,
    ) -> Option<(search::DartType, PathBuf)> {
        let line_of = |name: &str| {
            let bindings = search::bindings(Kind::Dart, text, self.line + 1, name);
            let [b] = bindings.as_slice() else {
                return None;
            };
            text.lines().nth(b.line1 - 1).map(|l| (l, b.line1))
        };
        let (line, line1) = line_of(name)?;
        let next: usize = text.split_inclusive('\n').take(line1).map(str::len).sum();
        if let Some(ty) = search::dart_typed(line, text.get(next..)?, name) {
            let ty = search::dart_type_bound(line, &ty).unwrap_or(ty);
            return Some((ty, here.to_path_buf()));
        }
        if let Some(list) = search::dart_list_element(line, name) {
            let (line, _) = line_of(list).filter(|_| list != name)?;
            return Some((search::dart_element_typed(line, list)?, here.to_path_buf()));
        }
        if let Some(items) = search::dart_for_in(line, name) {
            if items == name {
                return None;
            }
            if let Some((line, _)) = line_of(items) {
                return Some((search::dart_iterated(line, items)?, here.to_path_buf()));
            }
            let c = self.dart_field_row(here, text, items)?;
            return Some((search::dart_iterated(&c.hit.text, items)?, c.hit.path));
        }
        let value = search::dart_value(line, name).filter(|_| depth < 4)?;
        if let Some((receiver, getter)) = search::dart_awaited(value) {
            return self.dart_awaited(here, text, receiver, getter, depth + 1);
        }
        let value = format!("{value}.");
        let (links, _, start) = search::dart_call_links(&value)?;
        let lead = value.get(..start)?.trim();
        (links[0] != name && matches!(lead, "" | "new" | "const"))
            .then(|| self.dart_chain_type(here, text, &value, depth + 1))?
    }

    fn dart_awaited(
        &self,
        here: &Path,
        text: &str,
        receiver: &str,
        getter: &str,
        depth: usize,
    ) -> Option<(search::DartType, PathBuf)> {
        let (ty, scope) = self.dart_bound(here, text, receiver, depth)?;
        let file = self
            .dart_class_file(&scope, &ty)
            .or_else(|| self.dart_package_class_file(&scope, &ty))?;
        let [row] = self
            .dart_member_of(&file, &ty.class, getter)
            .try_into()
            .ok()?;
        let awaited = search::dart_future_of(&row.hit.text, getter, false)?;
        let class_text = self.text_of(&file)?;
        let params = search::dart_type_params(&class_text, &ty.class)?;
        if awaited.prefix.is_some() || !params.contains(&awaited.class) {
            return Some((awaited, row.hit.path));
        }
        let ctor = ty.constructor.as_deref()?;
        let named = format!("{}.{ctor}", ty.class);
        let made: Vec<Candidate> = self
            .dart_member_of(&file, &ty.class, ctor)
            .into_iter()
            .filter(|c| c.hit.text.contains(&named))
            .collect();
        let [made] = made.as_slice() else {
            return None;
        };
        let decl = class_text.get(line_offset(&class_text, made.hit.line1)..)?;
        let open = Regex::new(&format!(r"{}\s*\(", regex::escape(&named)))
            .ok()?
            .find(decl)?
            .end()
            - 1;
        if !search::dart_future_param(decl, open, &awaited.class) {
            return None;
        }
        let [b] = search::bindings(Kind::Dart, text, self.line + 1, receiver)
            .try_into()
            .ok()?;
        let arg =
            search::dart_constructed_arg(text.get(line_offset(text, b.line1)..)?, receiver, &ty)?;
        let (on, method) = search::dart_method_call(arg)?;
        let (on_ty, on_scope) =
            self.dart_links_type(here, text, &[on.to_owned()], false, false, depth)?;
        let on_file = self.dart_class_file(&on_scope, &on_ty)?;
        let [call] = self
            .dart_member_of(&on_file, &on_ty.class, method)
            .try_into()
            .ok()?;
        let result = search::dart_future_of(&call.hit.text, method, true)?;
        let on_params = search::dart_type_params(&self.text_of(&on_file)?, &on_ty.class)?;
        (result.prefix.is_some() || !on_params.contains(&result.class))
            .then_some((result, call.hit.path))
    }

    fn dart_package_class_file(&self, here: &Path, ty: &search::DartType) -> Option<PathBuf> {
        let sdk = self.dart_sdk()?;
        let files = self.dart_visible_files(here, &ty.class, ty.prefix.as_deref())?;
        if files
            .iter()
            .any(|f| !f.is_absolute() && self.dart_declares_class(f, &ty.class))
        {
            return None;
        }
        let (library, directives) = self.dart_library(here.to_path_buf(), Some(&sdk), false)?;
        let mut declaring = directives
            .iter()
            .filter(|d| {
                d.keyword == "import"
                    && d.prefix == ty.prefix
                    && d.uri.starts_with("package:")
                    && dart_names(d, &ty.class)
            })
            .filter_map(|d| self.dart_sdk_exported(&library[0], &d.uri, &ty.class, &sdk, 0))
            .filter(|f| f.is_absolute());
        let file = declaring.next()?;
        declaring.all(|f| f == file).then_some(file)
    }

    fn dart_chain_type(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        depth: usize,
    ) -> Option<(search::DartType, PathBuf)> {
        let (links, called, _) = search::dart_call_links(before)?;
        self.dart_links_type(here, text, &links, called, false, depth)
    }

    fn dart_links_type(
        &self,
        here: &Path,
        text: &str,
        links: &[String],
        called: bool,
        plain: bool,
        depth: usize,
    ) -> Option<(search::DartType, PathBuf)> {
        let (head, rest) = links.split_first()?;
        let (mut ty, mut scope) = if called {
            self.dart_call_type(here, text, head)?
        } else {
            match self.dart_bound(here, text, head, depth) {
                Some(found) => found,
                None => self.dart_field_type(here, text, head)?,
            }
        };
        for link in rest {
            let file = self.dart_class_file(&scope, &ty)?;
            let mut rows = self.dart_member_of(&file, &ty.class, link);
            if rows.is_empty() {
                rows = self.dart_inherited(&file, &ty.class, link, 0);
            }
            let [row] = rows.as_slice() else {
                return None;
            };
            let method = Regex::new(&format!(r"\b{}\s*(?:<[^()]*>)?\s*\(", regex::escape(link)));
            if plain && method.is_ok_and(|m| m.is_match(&row.hit.text)) {
                return None;
            }
            ty = search::dart_returns(&row.hit.text, link)
                .or_else(|| search::dart_typed(&row.hit.text, "", link))?;
            scope.clone_from(&row.hit.path);
        }
        Some((ty, scope))
    }

    fn dart_call_type(
        &self,
        here: &Path,
        text: &str,
        name: &str,
    ) -> Option<(search::DartType, PathBuf)> {
        if !search::bindings(Kind::Dart, text, self.line + 1, name).is_empty() {
            return None;
        }
        if name.trim_start_matches('_').starts_with(char::is_uppercase) {
            let ty = search::DartType {
                prefix: None,
                class: name.to_owned(),
                constructor: None,
            };
            return Some((ty, here.to_path_buf()));
        }
        if search::dart_body_at(text, self.line).is_none_or(|(body, _)| !body.is_empty()) {
            return None;
        }
        let reason = Reason::Receiver(name.to_owned());
        let found: Vec<Candidate> = self
            .dart_visible_files(here, name, None)?
            .iter()
            .flat_map(|f| self.dart_declared_in(f, name, None, &[None], &reason))
            .collect();
        let [c] = found.as_slice() else {
            return None;
        };
        let ty = search::dart_returns(&c.hit.text, name)
            .filter(|_| !c.hit.text.starts_with(char::is_whitespace))?;
        Some((ty, c.hit.path.clone()))
    }

    fn dart_accessor_side(&self, before: &str, word: &str, found: &mut Vec<Candidate>) {
        static ASSIGNED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^\s*=(?:[^=>]|$)").expect("the regex is a literal")
        });
        static READ: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"^\s*(?:$|[),;\]}.]|[!=]=|&&|\|\|)").expect("the regex is a literal")
        });
        let Some(tail) = self
            .word_here(Some(Kind::Dart))
            .and_then(|(range, _)| self.line_str().get(range.end..))
        else {
            return;
        };
        let accessor = |c: &Candidate, side: &str| {
            Regex::new(&format!(r"\b{side}\s+{}(?:[^\w$]|$)", regex::escape(word)))
                .is_ok_and(|r| r.is_match(&c.hit.text))
        };
        let [a, b] = found.as_slice() else {
            return;
        };
        let pair = a.hit.path == b.hit.path
            && ((accessor(a, "get") && accessor(b, "set"))
                || (accessor(a, "set") && accessor(b, "get")));
        let operand = before
            .trim_end_matches(|c: char| c.is_alphanumeric() || "_$.?!".contains(c))
            .trim_end();
        let prefixed = operand.ends_with("++") || operand.ends_with("--");
        let side = if ASSIGNED.is_match(tail) {
            "set"
        } else if READ.is_match(tail) {
            "get"
        } else {
            return;
        };
        if pair && !prefixed {
            found.retain(|c| accessor(c, side));
        }
    }

    fn dart_own_member(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let (range, _) = self.word_here(Some(Kind::Dart))?;
        let tail = self.line_str().get(range.end..)?;
        let called = tail.starts_with('(');
        if !chain.is_empty()
            || before.trim_end().ends_with('.')
            || (tail.starts_with(':') && search::dart_label(text, self.line + 1, range.start))
            || !search::dart_bindings_at(text, self.line + 1, range.start, word).is_empty()
        {
            return None;
        }
        let own = search::dart_this_super(text, self.line, false)?;
        let mut found = self.dart_member_of(here, &own.class, word);
        if !called {
            let declares = Regex::new(&search::def_patterns(Kind::Dart, word).join("|"))
                .is_ok_and(|d| d.is_match(self.line_str()));
            if declares {
                return None;
            }
            self.dart_accessor_side(before, word, &mut found);
            if found.len() != 1 {
                return None;
            }
        }
        (own.class != word
            && !found.is_empty()
            && found.iter().all(|c| c.hit.line1 != self.line + 1))
        .then_some(found)
    }

    fn dart_core_unreached(&self, here: &Path, ty: &search::DartType, word: &str) -> bool {
        let class = ty.class.as_str();
        let on = Regex::new(r"^extension(?:\s+(\w+))?(?:\s*<[^>]*>)?\s+on\s+(\w+)");
        let declares = Regex::new(&search::def_patterns(Kind::Dart, word).join("|"));
        let (Ok(on), Ok(declares)) = (on, declares) else {
            return false;
        };
        let reaches = |target: &str| {
            target == class
                || target == "Object"
                || !search::DART_CORE_TYPES.contains(&target)
                || (target == "Iterable" && matches!(class, "List" | "Set"))
                || (target == "num" && matches!(class, "int" | "double"))
        };
        let extends = |t: &str| {
            t.lines()
                .filter_map(|l| on.captures(l))
                .filter(|c| reaches(&c[2]))
                .any(|c| {
                    let within = c.get(1).map(|n| format!("{}.{word}", n.as_str()));
                    t.lines().enumerate().any(|(i, l)| {
                        declares.is_match(l)
                            && within.as_ref().is_none_or(|w| {
                                search::qualified(Kind::Dart, t, i + 1, word).as_ref() == Some(w)
                            })
                    })
                })
        };
        ty.prefix.is_none()
            && search::DART_CORE_TYPES.contains(&class)
            && self
                .dart_visible_files(here, class, None)
                .is_some_and(|files| {
                    files.iter().filter(|f| !f.is_absolute()).all(|f| {
                        !self.dart_declares_class(f, class)
                            && self.text_of(f).is_none_or(|t| !extends(&t))
                    })
                })
    }

    fn dart_field_type(
        &self,
        here: &Path,
        text: &str,
        field: &str,
    ) -> Option<(search::DartType, PathBuf)> {
        let c = self.dart_field_row(here, text, field)?;
        let ty = search::dart_typed(&c.hit.text, "", field)?;
        Some((ty, c.hit.path))
    }

    fn dart_field_row(&self, here: &Path, text: &str, field: &str) -> Option<Candidate> {
        if !search::bindings(Kind::Dart, text, self.line + 1, field).is_empty() {
            return None;
        }
        let class = search::dart_this_super(text, self.line, false)?.class;
        let mut found = self.dart_member_of(here, &class, field);
        if found.is_empty() {
            if Self::dart_top_level(text, field) {
                return None;
            }
            found = self.dart_inherited(here, &class, field, 0);
            if field.starts_with('_') {
                let (library, _) = self.dart_library(here.to_path_buf(), None, false)?;
                found.retain(|c| library.contains(&c.hit.path));
            }
        }
        let [c] = found.try_into().ok()?;
        Some(c)
    }

    fn dart_top_level(text: &str, name: &str) -> bool {
        let Ok(declares) = Regex::new(&search::def_patterns(Kind::Dart, name).join("|")) else {
            return false;
        };
        let literal = search::literal_lines(Kind::Dart, text);
        text.lines().enumerate().any(|(i, l)| {
            !literal[i] && !l.starts_with(char::is_whitespace) && declares.is_match(l)
        })
    }

    fn dart_member_of(&self, file: &Path, class: &str, word: &str) -> Vec<Candidate> {
        let bare = Some(search::dart_bare_constructor(class, word));
        let within = [Some(format!("{class}.{word}"))];
        let reason = Reason::Receiver(class.to_owned());
        self.dart_declared_in(file, word, bare, &within, &reason)
    }

    fn dart_inherited(&self, file: &Path, class: &str, word: &str, depth: usize) -> Vec<Candidate> {
        let Some(text) = self.text_of(file).filter(|_| depth < 8) else {
            return Vec::new();
        };
        for ty in search::dart_supertypes(&text, class) {
            let Some(up) = self.dart_class_file(file, &ty) else {
                return Vec::new();
            };
            let mut found = self.dart_member_of(&up, &ty.class, word);
            if found.is_empty() {
                found = self.dart_inherited(&up, &ty.class, word, depth + 1);
            }
            if !found.is_empty() {
                return found;
            }
        }
        Vec::new()
    }

    fn dart_class_file(&self, here: &Path, ty: &search::DartType) -> Option<PathBuf> {
        let files = self.dart_visible_files(here, &ty.class, ty.prefix.as_deref())?;
        let mut declaring = files
            .into_iter()
            .filter(|f| !f.is_absolute() && self.dart_declares_class(f, &ty.class));
        match (declaring.next(), declaring.next()) {
            (Some(file), None) => Some(file),
            (None, _) if ty.prefix.is_none() => self.dart_sdk_class_file(here, &ty.class),
            _ => None,
        }
    }

    fn dart_declares_class(&self, file: &Path, class: &str) -> bool {
        let Ok(class) = Regex::new(&search::def_patterns(Kind::Dart, class)[0]) else {
            return false;
        };
        self.text_of(file).is_some_and(|t| {
            let literal = search::literal_lines(Kind::Dart, &t);
            t.lines().enumerate().any(|(i, l)| {
                !literal[i] && !l.starts_with(char::is_whitespace) && class.is_match(l)
            })
        })
    }

    fn dart_sdk(&self) -> Option<PathBuf> {
        self.external
            .get(&Kind::Dart)
            .and_then(|(roots, _)| {
                roots
                    .iter()
                    .find(|r| r.join("core/core.dart").is_file())
                    .and_then(|r| r.parent())
                    .map(Path::to_path_buf)
            })
            .or_else(search::dart_sdk)
    }

    fn dart_sdk_class_file(&self, here: &Path, class: &str) -> Option<PathBuf> {
        let sdk = self.dart_sdk()?;
        let (library, directives) = self.dart_library(here.to_path_buf(), Some(&sdk), false)?;
        if here.is_absolute()
            && let Some(own) = library.iter().find(|f| self.dart_declares_class(f, class))
        {
            return Some(own.clone());
        }
        let implicit_core = !directives
            .iter()
            .any(|d| d.keyword == "import" && d.uri == "dart:core");
        let uris = directives
            .iter()
            .filter(|d| {
                d.keyword == "import"
                    && d.prefix.is_none()
                    && d.uri.starts_with("dart:")
                    && dart_names(d, class)
            })
            .map(|d| d.uri.as_str())
            .chain(implicit_core.then_some("dart:core"));
        let mut declaring =
            uris.filter_map(|uri| self.dart_sdk_exported(&library[0], uri, class, &sdk, 0));
        let file = declaring.next()?;
        declaring.all(|f| f == file).then_some(file)
    }

    fn dart_sdk_exported(
        &self,
        from: &Path,
        uri: &str,
        class: &str,
        sdk: &Path,
        depth: usize,
    ) -> Option<PathBuf> {
        let file = search::dart_uri_file(&self.root, from, uri, Some(sdk))?;
        let (files, directives) = self.dart_library(file, Some(sdk), false)?;
        if let Some(f) = files.iter().find(|f| self.dart_declares_class(f, class)) {
            return Some(f.clone());
        }
        directives
            .iter()
            .filter(|d| depth < 4 && d.keyword == "export" && dart_names(d, class))
            .find_map(|d| self.dart_sdk_exported(&files[0], &d.uri, class, sdk, depth + 1))
    }
}

fn dart_names(d: &search::DartDirective, name: &str) -> bool {
    d.show.as_ref().is_none_or(|s| s.iter().any(|n| n == name)) && !d.hide.iter().any(|n| n == name)
}

fn line_offset(text: &str, line1: usize) -> usize {
    text.split_inclusive('\n')
        .take(line1.saturating_sub(1))
        .map(str::len)
        .sum()
}
