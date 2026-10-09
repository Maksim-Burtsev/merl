use super::*;

struct CTypeIn {
    file: PathBuf,
    ty: search::CType,
}

impl App {
    pub(super) fn c_early(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        range: std::ops::Range<usize>,
    ) -> Option<(Vec<Candidate>, Option<String>)> {
        static TRAILING: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?:\)|\b(?:const|noexcept|mutable|override))\s+->\s*$").unwrap()
        });
        let line = self.line_str();
        let (before, after) = (
            line[..range.start].trim_end(),
            line[range.end..].trim_start(),
        );
        let called = after.starts_with('(');
        let member = (before.ends_with("->") && !TRAILING.is_match(before))
            || (before.ends_with('.') && !before.ends_with(".."));
        if member
            && word.starts_with(|c: char| c.is_alphabetic() || c == '_')
            && !line.trim_start().starts_with('#')
        {
            let before = before.to_owned();
            let mut broke = match self.c_receiver_field(here, text, &before, word) {
                Ok(Some(found)) => return Some((vec![found], None)),
                Ok(None) => None,
                Err(at) => Some(at),
            };
            if !self.c_source() && !self.objc_file() {
                match self.cpp_typed(here, text, &before, word) {
                    cpp::CppAnswer::Found(found) => return Some((found, None)),
                    cpp::CppAnswer::Outside => {
                        return Some((self.c_members(here, word, called, true), None));
                    }
                    cpp::CppAnswer::ByName(at) => broke = broke.or(at),
                }
            }
            return Some((self.c_members(here, word, called, false), broke));
        }
        let class = after
            .starts_with(['(', '{'])
            .then(|| search::c_initialized_member(text, self.line + 1, range.start))
            .flatten()?;
        let found = self.c_class_fields(here, word, &class, true);
        (!found.is_empty()).then_some((found, None))
    }

    fn c_receiver_field(
        &self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
    ) -> Result<Option<Candidate>, String> {
        let c = here.extension().is_some_and(|e| e == "c" || e == "h");
        let Some(search::CReceiver {
            head,
            head_called: called,
            fields,
        }) = c.then(|| search::c_receiver(before)).flatten()
        else {
            return Ok(None);
        };
        let broke = |at: &str| match fields.is_empty() {
            true => Ok(None),
            false => Err(at.to_owned()),
        };
        let start = match called {
            true => self.c_return_type(here, &head).map(|t| CTypeIn {
                file: here.to_path_buf(),
                ty: search::CType::Name(t),
            }),
            false => self.c_head_type(here, text, &head),
        };
        let Some(mut ty) = start else {
            return broke(&head);
        };
        let mut links = Vec::new();
        let mut prev = head.clone();
        for (i, name) in fields.iter().chain([&word.to_owned()]).enumerate() {
            let Some(body) = self.c_body(here, &ty) else {
                return broke(&prev);
            };
            let link = match &ty.ty {
                search::CType::Name(t) if i == 0 && called => format!("{prev}(): {t}"),
                search::CType::Name(t) => format!("{prev}: {t}"),
                search::CType::Body(open) => match search::c_body_name(&body.code, *open) {
                    t if t.is_empty() => prev.clone(),
                    t => format!("{prev}: {t}"),
                },
            };
            links.push(link);
            let Some(at) = search::c_body_field(&body.code, body.brace_in_code, name) else {
                return match i == fields.len() {
                    true => Ok(None),
                    false => broke(name),
                };
            };
            if i == fields.len() {
                let search::CPlace {
                    line1: line,
                    byte_col: col,
                } = search::c_place(&body.code, at);
                let hit = Hit {
                    path: body.path,
                    line1: line,
                    byte_col: Some(col),
                    text: body
                        .text
                        .lines()
                        .nth(line - 1)
                        .unwrap_or_default()
                        .to_owned(),
                    deleted: None,
                };
                let reason = Reason::Receiver(links.join(" \u{2192} "));
                return Ok(Some(Candidate { hit, reason }));
            }
            let Some(next) = search::c_decl_type(&body.code, at) else {
                return broke(name);
            };
            ty = CTypeIn {
                file: body.path,
                ty: next,
            };
            prev = name.clone();
        }
        Ok(None)
    }

    fn c_head_type(&self, here: &Path, text: &str, name: &str) -> Option<CTypeIn> {
        match search::c_value_type(text, self.line + 1, name) {
            Ok(Some(ty)) => {
                return Some(CTypeIn {
                    file: here.to_path_buf(),
                    ty,
                });
            }
            Ok(None) => {}
            Err(()) => return None,
        }
        let p = search::def_patterns(Kind::C, name);
        let hits = self.project_definitions(Kind::C, here, name, &p[8]);
        let [h] = hits
            .iter()
            .filter(|h| c_header(&h.path))
            .collect::<Vec<_>>()[..]
        else {
            return None;
        };
        let code = search::c_code(&self.text_of(&h.path)?);
        let t = search::c_words_on_line(&code, h.line1, name, None)
            .into_iter()
            .find_map(|at| search::c_decl_type(&code, at))?;
        matches!(t, search::CType::Name(_)).then(|| CTypeIn {
            file: h.path.clone(),
            ty: t,
        })
    }

    fn c_return_type(&self, here: &Path, name: &str) -> Option<String> {
        let p = search::def_patterns(Kind::C, name);
        let hits = self.project_definitions(Kind::C, here, name, &format!("{}|{}", p[0], p[7]));
        let mut found: Option<String> = None;
        for h in &hits {
            if h.text.trim_start().starts_with('#') {
                return None;
            }
            let code = search::c_code(&self.text_of(&h.path)?);
            let t = search::c_words_on_line(&code, h.line1, name, Some('('))
                .into_iter()
                .find_map(|at| search::c_decl_type(&code, at));
            let Some(search::CType::Name(t)) = t else {
                return None;
            };
            if found.as_ref().is_some_and(|f| *f != t) {
                return None;
            }
            found = Some(t);
        }
        found
    }

    fn c_body(&self, here: &Path, ty: &CTypeIn) -> Option<CBody> {
        let read = |path: &Path, open: usize| {
            let text = self.text_of(path)?;
            let code = search::c_code(&text);
            Some(CBody {
                path: path.to_path_buf(),
                text,
                code,
                brace_in_code: open,
            })
        };
        let name = match &ty.ty {
            search::CType::Body(open) => return read(&ty.file, *open),
            search::CType::Name(name) => name,
        };
        let mut bodies = Vec::new();
        self.c_bodies(here, name, 0, &mut bodies);
        if bodies.len() > 1 {
            bodies.retain(|b| b.path == here || c_header(&b.path));
        }
        match bodies.as_slice() {
            [body] => read(&body.path, body.byte),
            _ => None,
        }
    }

    fn c_bodies(&self, here: &Path, name: &str, depth: usize, bodies: &mut Vec<BraceAt>) {
        let p = search::def_patterns(Kind::C, name);
        let pattern = [&p[2], &p[4], &p[5]].map(String::as_str).join("|");
        let mut aliases = Vec::new();
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        for h in self.project_definitions(Kind::C, here, name, &pattern) {
            if !files.iter().any(|(p, _)| *p == h.path) {
                let Some(text) = self.text_of(&h.path) else {
                    continue;
                };
                files.push((h.path.clone(), search::c_code(&text)));
            }
            let code = &files
                .iter()
                .find(|(p, _)| *p == h.path)
                .expect("pushed above")
                .1;
            let def = search::c_words_on_line(code, h.line1, name, None)
                .into_iter()
                .find_map(|at| search::c_type_def(code, at));
            match def {
                Some(search::CType::Body(byte))
                    if !bodies.iter().any(|b| b.path == h.path && b.byte == byte) =>
                {
                    bodies.push(BraceAt { path: h.path, byte })
                }
                Some(search::CType::Name(alias)) if !aliases.contains(&alias) => {
                    aliases.push(alias)
                }
                _ => {}
            }
        }
        if depth < 3 {
            for alias in aliases {
                self.c_bodies(here, &alias, depth + 1, bodies);
            }
        }
    }

    pub(super) fn c_members(
        &mut self,
        here: &Path,
        word: &str,
        called: bool,
        outside: bool,
    ) -> Vec<Candidate> {
        let mut pattern = search::c_field_pattern(word);
        if self.objc_file() {
            pattern = format!("{pattern}|{}", search::objc_member(word));
        }
        if called {
            pattern = format!(
                r"{pattern}|{}|{}",
                method_pattern(word),
                search::c_member_decl(Some(word))
            );
        }
        let hits = match outside {
            true => Vec::new(),
            false => self.project_definitions(Kind::C, here, word, &pattern),
        };
        let mut found = self.c_member_rows(word, hits, called);
        let property = Regex::new(&search::objc_property(word)).expect("an escaped name");
        found.retain(|MemberRow { hit: h, .. }| {
            h.path == here
                || !property.is_match(&h.text)
                || !h.path.extension().is_some_and(|e| e == "m" || e == "mm")
        });
        if found.is_empty() {
            let files = self.external_files(Kind::C);
            found = self.c_outside(
                here,
                &files,
                |row| &row.hit,
                |this, files| {
                    let hits = this.external_grep(Kind::C, files, &pattern);
                    this.c_member_rows(word, hits, called)
                },
            );
        }
        found
            .into_iter()
            .map(|MemberRow { hit, .. }| Candidate {
                hit,
                reason: Reason::ByName,
            })
            .collect()
    }

    fn c_member_rows(&self, word: &str, hits: Vec<Hit>, called: bool) -> Vec<MemberRow> {
        let w = regex::escape(word);
        let re = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
        let method = re(method_pattern(word));
        let declared = re(search::c_member_decl(Some(word)));
        let pointer = re(format!(r"\(\s*\*+\s*{w}\s*\)"));
        let (member, objc) = (re(search::objc_member(word)), self.objc_file());
        let mut rows = Vec::new();
        let mut files: Vec<(PathBuf, Vec<Hit>)> = Vec::new();
        for h in hits {
            match files.iter_mut().find(|(p, _)| *p == h.path) {
                Some((_, hs)) => hs.push(h),
                None => files.push((h.path.clone(), vec![h])),
            }
        }
        for (path, hits) in files {
            let Some(text) = self.text_of(&path) else {
                continue;
            };
            let lines: Vec<usize> = hits.iter().map(|h| h.line1).collect();
            let fields = search::c_field_rows(&text, &lines, word);
            for h in hits {
                match fields.iter().find(|f| f.line1 == h.line1) {
                    Some(search::CFieldRow {
                        owner_type: owner, ..
                    }) if !called || pointer.is_match(&h.text) => {
                        rows.push(MemberRow::field_of(h, owner))
                    }
                    None if called && method.is_match(&h.text) => rows.push(MemberRow::method(h)),
                    None if objc && member.is_match(&h.text) => rows.push(MemberRow::method(h)),
                    None if called
                        && declared.is_match(&h.text)
                        && search::c_member_class(&text, h.line1, word).is_some() =>
                    {
                        rows.push(MemberRow::method(h))
                    }
                    _ => {}
                }
            }
        }
        rows
    }

    pub(super) fn c_class_fields(
        &mut self,
        here: &Path,
        word: &str,
        class: &str,
        by_name: bool,
    ) -> Vec<Candidate> {
        let hits = self.project_definitions(Kind::C, here, word, &search::c_field_pattern(word));
        let rows = self.c_member_rows(word, hits, false);
        let own = rows.iter().any(|row| row.is_field_of(class));
        rows.into_iter()
            .filter(|row| row.is_field_of(class) || (by_name && !own))
            .map(|MemberRow { hit, .. }| Candidate {
                hit,
                reason: match own {
                    true => Reason::Receiver(class.to_owned()),
                    false => Reason::ByName,
                },
            })
            .collect()
    }

    pub(super) fn c_reached(&mut self, here: &Path) -> HashSet<PathBuf> {
        self.external_files(Kind::C);
        let mode = (self.c_source(), self.objc_file());
        let source = mode.0;
        let all = (self.external.get(&Kind::C))
            .map(|(roots, _)| roots.clone())
            .unwrap_or_default();
        let (frameworks, roots): (Vec<PathBuf>, Vec<PathBuf>) =
            (all.into_iter()).partition(|r| search::objc_frameworks(r));
        let frameworks = if mode.1 { frameworks } else { Vec::new() };
        let roots: Vec<PathBuf> = (roots.into_iter())
            .filter(|r| mode.1 || !search::objc_root(r))
            .collect();
        let dirs: Vec<PathBuf> = match source {
            true => roots.clone(),
            false => [search::cpp_dirs(&roots), roots.clone()].concat(),
        };
        let real = search::real_dirs(&roots);
        let own: HashSet<&PathBuf> = self.files.iter().collect();
        let mut by_name: HashMap<&std::ffi::OsStr, Vec<&PathBuf>> = HashMap::new();
        for f in &self.files {
            if let Some(name) = f.file_name() {
                by_name.entry(name).or_default().push(f);
            }
        }
        let edges = |text: &str, from: &Path| -> Vec<PathBuf> {
            let mut out = Vec::new();
            for search::CInclude {
                as_written: inc,
                quoted_not_angled: quoted,
            } in search::c_includes(text)
            {
                if inc.starts_with("__cxx03/") {
                    continue;
                }
                let inc = Path::new(&inc);
                let beside = from.parent().map(|d| normal(&d.join(inc)));
                match beside {
                    Some(b) if quoted && from.is_relative() && own.contains(&b) => {
                        out.push(b);
                        continue;
                    }
                    Some(b) if quoted && from.is_absolute() && b.is_file() => {
                        out.push(search::c_spelled(&b, &real));
                        continue;
                    }
                    _ => {}
                }
                let n = out.len();
                if !quoted {
                    // ponytail: every directory that has it, not the first: an `#include_next`
                    // reaches the next one, and a wrapper of libc++ (`stdio.h`) the C one.
                    out.extend(
                        (dirs.iter().map(|d| d.join(inc)))
                            .filter(|p| p.is_file())
                            .map(|p| search::c_spelled(&p, &real)),
                    );
                    let mut parts = inc.components();
                    if let (Some(name), rest) = (parts.next(), parts.as_path()) {
                        let mut name = name.as_os_str().to_owned();
                        name.push(".framework");
                        out.extend(
                            (frameworks.iter())
                                .map(|d| d.join(&name).join("Headers").join(rest))
                                .filter(|p| p.is_file()),
                        );
                    }
                }
                if out.len() == n {
                    let suffix = inc.file_name().and_then(|n| by_name.get(n));
                    let own = suffix.into_iter().flatten().filter(|f| f.ends_with(inc));
                    out.extend(own.map(|f| (*f).clone()));
                }
            }
            out
        };
        let mut reached = HashSet::new();
        let mut queue = vec![here.to_path_buf()];
        while let Some(file) = queue.pop() {
            let next = match self.c_includes.get(&(file.clone(), mode)) {
                Some(next) if file.is_absolute() => next.clone(),
                _ => {
                    let text = match file.is_absolute() {
                        true => std::fs::read_to_string(&file).ok(),
                        false => self.text_of(&file),
                    };
                    let next = Arc::new(edges(&text.unwrap_or_default(), &file));
                    if file.is_absolute() {
                        self.c_includes.insert((file, mode), next.clone());
                    }
                    next
                }
            };
            for f in next.iter() {
                if reached.insert(f.clone()) {
                    queue.push(f.clone());
                }
            }
        }
        reached
    }

    pub(super) fn c_outside<T>(
        &mut self,
        here: &Path,
        files: &[PathBuf],
        hit: fn(&T) -> &Hit,
        search: impl Fn(&Self, &[PathBuf]) -> Vec<T>,
    ) -> Vec<T> {
        let reached = self.c_reached(here);
        let (near, far): (Vec<PathBuf>, Vec<PathBuf>) =
            files.iter().cloned().partition(|f| reached.contains(f));
        let mut literal = HashMap::new();
        let found: Vec<T> = (search(self, &near).into_iter())
            .filter(|f| self.c_code_line(&mut literal, hit(f)))
            .collect();
        match found.is_empty() {
            true => search(self, &far),
            false => found,
        }
    }

    pub(super) fn c_code_line(
        &self,
        literal_lines: &mut HashMap<PathBuf, Vec<bool>>,
        h: &Hit,
    ) -> bool {
        let lines = literal_lines.entry(h.path.clone()).or_insert_with(|| {
            (self.text_of(&h.path)).map_or_else(Vec::new, |t| search::literal_lines(Kind::C, &t))
        });
        !lines.get(h.line1 - 1).copied().unwrap_or(false)
    }

    pub(super) fn c_reached_only(
        &mut self,
        here: &Path,
        word: &str,
        mut hits: Vec<Hit>,
    ) -> Vec<Hit> {
        let header = |h: &Hit| h.path != here && c_header(&h.path);
        if !hits.iter().any(header) {
            return hits;
        }
        let reached = self.c_reached(here);
        let defines = |h: &Hit| {
            !(self.text_of(&h.path)).is_some_and(|t| search::c_declaration_only(word, &t, h.line1))
        };
        let (near, far): (Vec<&Hit>, Vec<&Hit>) = (hits.iter())
            .filter(|h| header(h))
            .partition(|h| reached.contains(&h.path));
        if near.is_empty() {
            return hits;
        }
        let defined = near.iter().any(|h| defines(h));
        let gone: Vec<(PathBuf, usize)> = (far.into_iter())
            .filter(|h| defined || !defines(h))
            .map(|h| (h.path.clone(), h.line1))
            .collect();
        hits.retain(|h| !gone.contains(&(h.path.clone(), h.line1)));
        hits
    }

    pub(super) fn c_rows(&self, word: &str, here: &Path, found: &mut Vec<Candidate>) {
        if found.len() > 1 {
            let classes: Vec<Option<Vec<String>>> = found
                .iter()
                .map(|c| {
                    self.text_of(&c.hit.path)
                        .and_then(|t| search::c_member_class(&t, c.hit.line1, word))
                })
                .collect();
            let defined = |scopes: &[String]| {
                found.iter().any(|c| {
                    (c.hit.line1 != self.line + 1 || c.hit.path != here)
                        && search::c_defines_member(&c.hit.text, scopes, word)
                })
            };
            let keep: Vec<bool> = classes
                .iter()
                .map(|class| class.as_deref().is_none_or(|c| !defined(c)))
                .collect();
            let mut keep = keep.into_iter();
            found.retain(|_| keep.next().unwrap_or(true));
        }
        static OBJC: std::sync::LazyLock<Regex> =
            std::sync::LazyLock::new(|| Regex::new(r"^\s*@(interface|protocol)\b").unwrap());
        let objc = |c: &Candidate| OBJC.captures(&c.hit.text).map(|m| &m[1] == "protocol");
        if !found.iter().any(|c| objc(c).is_some()) {
            return;
        }
        found.retain(|c| objc(c).is_some());
        let line = self.line_str();
        let (before, after) = line.split_at(self.col.min(line.len()));
        let after = after.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_');
        let protocol = before.matches('<').count() > before.matches('>').count()
            && !after.trim_start().starts_with('*');
        if found.iter().any(|c| objc(c) == Some(protocol)) {
            found.retain(|c| objc(c) == Some(protocol));
        }
    }

    pub(super) fn c_type_line(word: &str, hit: &Hit) -> bool {
        let p = search::def_patterns(Kind::C, word);
        Regex::new(&[&p[2], &p[4], &p[6]].map(String::as_str).join("|"))
            .is_ok_and(|re| re.is_match(&hit.text))
    }
}

struct CBody {
    path: PathBuf,
    text: String,
    code: String,
    brace_in_code: usize,
}

struct BraceAt {
    path: PathBuf,
    byte: usize,
}

struct MemberRow {
    hit: Hit,
    field_of: Option<String>,
}

impl MemberRow {
    fn field_of(hit: Hit, owner: &str) -> Self {
        let field_of = Some(owner.to_owned());
        MemberRow { hit, field_of }
    }

    fn method(hit: Hit) -> Self {
        MemberRow {
            hit,
            field_of: None,
        }
    }

    fn is_field_of(&self, class: &str) -> bool {
        self.field_of.as_deref() == Some(class)
    }
}

fn normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

pub(super) fn c_header(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| ["h", "hh", "hpp", "hxx"].iter().any(|h| e == *h))
}

pub(super) fn method_pattern(word: &str) -> String {
    let w = regex::escape(word);
    format!(r"^\s+[^;(){{}}=]*\w[\s*&]+{w}\s*\([^;{{}}]*\)[^;{{}}=]*\{{|^\w[^;(){{}}=]*::{w}\s*\(")
}
