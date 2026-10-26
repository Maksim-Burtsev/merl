use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, Regex, is_word, search};
use std::ops::Range;

#[derive(Clone)]
struct PhpClass {
    path: PathBuf,
    text: String,
    header0: usize,
}

struct ProvenClass {
    class: PhpClass,
    link: String,
}

struct Returned {
    class: PhpClass,
    written: String,
}

enum Link {
    Class(PhpClass),
    Outside(String),
    Unread,
}

enum Walk {
    Found(Vec<Hit>),
    Nowhere,
    NowhereOutside,
    Unread,
}

impl App {
    pub(super) fn php_early(
        &mut self,
        here: &Path,
        text: &str,
        before: &str,
        chain: &[String],
        word: &str,
        range: Range<usize>,
    ) -> Option<Vec<Candidate>> {
        let dotted = before.ends_with('.') && !before.ends_with("..");
        let property = before.strip_suffix('$').filter(|b| b.ends_with("::"));
        let b = property.unwrap_or(before);
        let link = match search::qualifier(b, b.len()).as_slice() {
            [l] if dotted && l == "this" => Some(l.clone()),
            [l] if !dotted
                && b.ends_with("::")
                && matches!(l.as_str(), "self" | "static" | "parent") =>
            {
                Some(l.clone())
            }
            _ => None,
        };
        let access = match self
            .line_str()
            .get(range.end..)?
            .trim_start()
            .starts_with('(')
        {
            true => search::PhpAccess::Call,
            false if dotted || property.is_some() => search::PhpAccess::Property,
            false => search::PhpAccess::Constant,
        };
        if let Some(link) = link
            && let Some(found) = self.php_link(here, text, &link, word, access)
        {
            return Some(found);
        }
        let receiver = before.strip_suffix(&format!("${}.", chain.join(".")));
        if dotted
            && !chain.is_empty()
            && receiver.is_some_and(|b| !b.ends_with(|c: char| is_word(c) || c == ':' || c == '$'))
            && let Some(found) = self.php_typed(here, text, chain, word, access)
        {
            return Some(found);
        }
        if dotted && let Some(found) = self.php_called(here, text, before, word, access) {
            return Some(found);
        }
        if let Some(written) = self
            .line_str()
            .get(..range.start)
            .and_then(search::php_new_receiver)
        {
            let class = self.php_type(here, text, self.line, &written)?;
            let re = Regex::new(&search::php_access_patterns(word, access).join("|")).ok()?;
            let Walk::Found(hits) = self.php_walk(
                &class.path,
                &class.text,
                class.header0,
                &re,
                0,
                &mut Vec::new(),
            ) else {
                return None;
            };
            let reason = Reason::Receiver(format!("new {written}"));
            return Some(
                hits.into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: reason.clone(),
                    })
                    .collect(),
            );
        }
        if let Some(found) = self.php_function(here, text, range.clone()) {
            return Some(found);
        }
        self.php_named(here, text, range, access)
    }

    fn php_function(
        &mut self,
        here: &Path,
        text: &str,
        range: Range<usize>,
    ) -> Option<Vec<Candidate>> {
        let written = search::php_function_at(self.line_str(), range)?;
        let short = written.rsplit('\\').next().unwrap_or(&written);
        let pattern = format!(r"^\s*function\s+&?\s*{}\s*\(", regex::escape(short));
        let cut = self.truncated.get();
        let hits = self.project_definitions(Kind::Php, here, short, &pattern);
        let truncated = self.truncated.replace(cut);
        let names = search::php_function_names(search::php_block(text, self.line), &written);
        for (i, name) in names.iter().enumerate() {
            if i > 0 && truncated {
                return None;
            }
            let found = self.php_functions_in(&hits, name);
            if !found.is_empty() {
                return Some(found);
            }
        }
        if truncated {
            return None;
        }
        let vendor = self.root.join("vendor");
        let listing = std::fs::read_to_string(vendor.join("composer/autoload_files.php")).ok()?;
        let loaded = search::php_autoload_files(&listing, &vendor);
        let hits = self.outside_grep(Kind::Php, &loaded, &pattern, None);
        self.truncated.set(cut);
        if hits.iter().any(|h| search::php_polyfill(&h.path))
            || names
                .iter()
                .all(|n| self.php_functions_in(&hits, n).is_empty())
        {
            return None;
        }
        let files = self.external_files(Kind::Php);
        let hits = self.outside_grep(Kind::Php, &files, &pattern, None);
        if self.truncated.replace(cut) || hits.iter().any(|h| search::php_polyfill(&h.path)) {
            return None;
        }
        for name in &names {
            let found = self.php_functions_in(&hits, name);
            match found.len() {
                0 => {}
                1 => return Some(found),
                _ => return None,
            }
        }
        None
    }

    fn php_functions_in(&self, hits: &[Hit], name: &search::PhpResolved) -> Vec<Candidate> {
        let ns = name.full.rsplit_once('\\').map_or("", |(ns, _)| ns);
        let reason = match (name.imported, ns) {
            (true, _) => Reason::Import(name.full.clone()),
            (false, "") => Reason::ByName,
            (false, ns) => Reason::Path(ns.to_owned()),
        };
        hits.iter()
            .filter(|h| {
                let Some(t) = self.text_of(&h.path) else {
                    return false;
                };
                let at = h.line1 - 1;
                let lines: Vec<&str> = t.lines().collect();
                search::literal_lines(Kind::Php, &t).get(at) != Some(&true)
                    && search::php_enclosing_class(&lines, at).is_none()
                    && search::php_namespace(search::php_block(&t, at)).unwrap_or_default() == ns
            })
            .map(|h| Candidate {
                hit: h.clone(),
                reason: reason.clone(),
            })
            .collect()
    }

    fn php_named(
        &mut self,
        here: &Path,
        text: &str,
        range: Range<usize>,
        access: search::PhpAccess,
    ) -> Option<Vec<Candidate>> {
        let line = self.line_str();
        let word = line.get(range.clone())?.to_owned();
        let search::PhpClassAt { written, member } = search::php_class_at(line, range)?;
        let map = search::php_psr4(&self.root, here.parent().unwrap_or_else(|| Path::new("")));
        if map.is_empty() {
            return None;
        }
        let search::PhpResolved { full, imported } =
            search::php_resolve(search::php_block(text, self.line), &written)?;
        let (ns, short) = match full.rsplit_once('\\') {
            Some((ns, short)) => (ns, short),
            None => ("", full.as_str()),
        };
        let path = match search::php_psr4_file(&map, &full, |f| self.files.iter().any(|p| p == f)) {
            search::Psr4File::Found(path) => path,
            search::Psr4File::Missing => return None,
            search::Psr4File::OutsideProject => {
                let imports = [search::Import {
                    name: short.to_owned(),
                    path: full.split('\\').map(str::to_owned).collect(),
                }];
                let (word, chain) = if member {
                    (word, vec![short.to_owned()])
                } else {
                    (short.to_owned(), Vec::new())
                };
                let found: Vec<Candidate> = self
                    .external_definitions(Kind::Php, &word, &chain, false, &imports, true)?
                    .into_iter()
                    .filter(|c| {
                        c.reason.proven()
                            && std::fs::read_to_string(&c.hit.path)
                                .is_ok_and(|t| search::php_namespace(&t).unwrap_or_default() == ns)
                    })
                    .collect();
                return (!found.is_empty()).then_some(found);
            }
        };
        let t = self.text_of(&path)?;
        let lines: Vec<&str> = t.lines().collect();
        let literal = search::literal_lines(Kind::Php, &t);
        let class = (0..lines.len()).find(|&i| {
            literal.get(i) != Some(&true)
                && search::php_class_header(lines[i]).is_some_and(|h| h.name == short)
        })?;
        if search::php_namespace(search::php_block(&t, class)).unwrap_or_default() != ns {
            return None;
        }
        let reason = match imported {
            true => Reason::Import(path.display().to_string()),
            false if member || ns.is_empty() => Reason::Path(short.to_owned()),
            false => Reason::Path(ns.to_owned()),
        };
        let found = if member {
            let re = Regex::new(&search::php_access_patterns(&word, access).join("|")).ok()?;
            match self.php_walk(&path, &t, class, &re, 0, &mut Vec::new()) {
                Walk::Found(hits) => hits,
                Walk::Nowhere | Walk::NowhereOutside | Walk::Unread => return None,
            }
        } else {
            vec![Hit {
                deleted: None,
                path: path.clone(),
                line1: class + 1,
                byte_col: None,
                text: lines[class].to_owned(),
            }]
        };
        Some(
            found
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
        )
    }

    pub(super) fn php_link(
        &mut self,
        here: &Path,
        text: &str,
        link: &str,
        word: &str,
        access: search::PhpAccess,
    ) -> Option<Vec<Candidate>> {
        let lines: Vec<&str> = text.lines().collect();
        let class = search::php_enclosing_class(&lines, self.line)?;
        let search::PhpClassHeader {
            keyword,
            name,
            extends: parent,
        } = search::php_class_header(lines[class])?;
        if keyword == "trait" {
            return None;
        }
        let re = Regex::new(&search::php_access_patterns(word, access).join("|")).ok()?;
        let (reason, walk) = if link == "parent" {
            let parent = parent?;
            let reason = Reason::Receiver(format!("parent of {name}"));
            (
                reason,
                self.php_walk_class(
                    here,
                    search::php_block(text, class),
                    parent,
                    &re,
                    1,
                    &mut Vec::new(),
                ),
            )
        } else {
            let reason = Reason::Receiver(format!(
                "{}: {name}",
                if link == "this" { "$this" } else { link }
            ));
            let mut seen = Vec::new();
            (reason, self.php_walk(here, text, class, &re, 0, &mut seen))
        };
        match walk {
            Walk::Found(hits) => Some(
                hits.into_iter()
                    .map(|hit| Candidate {
                        hit,
                        reason: reason.clone(),
                    })
                    .collect(),
            ),
            Walk::NowhereOutside => Some(Vec::new()),
            Walk::Nowhere | Walk::Unread => None,
        }
    }

    fn php_walk(
        &mut self,
        path: &Path,
        text: &str,
        class: usize,
        re: &Regex,
        depth: usize,
        seen: &mut Vec<(PathBuf, usize)>,
    ) -> Walk {
        if depth > 8 || seen.contains(&(path.to_path_buf(), class)) {
            return Walk::Unread;
        }
        seen.push((path.to_path_buf(), class));
        let lines: Vec<&str> = text.lines().collect();
        let own: Vec<Hit> = search::php_class_members(&lines, class, re)
            .into_iter()
            .map(|i| Hit {
                deleted: None,
                path: path.to_path_buf(),
                line1: i + 1,
                byte_col: None,
                text: lines[i].to_owned(),
            })
            .collect();
        if !own.is_empty() {
            return Walk::Found(own);
        }
        let block = search::php_block(text, class);
        for a in search::php_trait_aliases(&lines, class) {
            if !re.is_match(&format!("function {}(", a.alias)) {
                continue;
            }
            let Ok(method) = Regex::new(
                &search::php_access_patterns(&a.method, search::PhpAccess::Call).join("|"),
            ) else {
                continue;
            };
            match self.php_walk_class(path, block, &a.from, &method, depth + 1, &mut seen.clone()) {
                Walk::Nowhere | Walk::NowhereOutside => {}
                other => return other,
            }
        }
        for t in search::php_class_traits(&lines, class) {
            match self.php_walk_class(path, block, &t, re, depth + 1, seen) {
                Walk::Nowhere | Walk::NowhereOutside => {}
                other => return other,
            }
        }
        let parent = search::php_class_header(lines[class])
            .or_else(|| search::php_anonymous_class(lines[class]))
            .and_then(|h| h.extends);
        match parent {
            Some(parent) => self.php_walk_class(path, block, parent, re, depth + 1, seen),
            None => Walk::Nowhere,
        }
    }

    fn php_walk_class(
        &mut self,
        from: &Path,
        text: &str,
        name: &str,
        re: &Regex,
        depth: usize,
        seen: &mut Vec<(PathBuf, usize)>,
    ) -> Walk {
        let (full, mut matching) = self.php_class(from, text, name);
        let short = full.as_deref().unwrap_or(name);
        let short = short.rsplit('\\').next().unwrap_or(short);
        match matching.len() {
            1 => {
                let class = matching.remove(0);
                self.php_walk(&class.path, &class.text, class.header0, re, depth, seen)
            }
            0 => self.php_outside_walk(full.as_deref(), short, re, depth, seen),
            _ => Walk::Unread,
        }
    }

    fn php_outside_walk(
        &mut self,
        full: Option<&str>,
        short: &str,
        re: &Regex,
        depth: usize,
        seen: &mut Vec<(PathBuf, usize)>,
    ) -> Walk {
        match self.php_outside_classes(full, short).as_slice() {
            [one] => match self.php_walk(&one.path, &one.text, one.header0, re, depth, seen) {
                Walk::Nowhere => Walk::NowhereOutside,
                other => other,
            },
            _ => self.php_outside(full, short, re),
        }
    }

    fn php_class(&self, from: &Path, text: &str, name: &str) -> (Option<String>, Vec<PhpClass>) {
        let kind = Kind::Php;
        let full = search::php_resolve(text, name).map(|r| r.full);
        let short = full.as_deref().unwrap_or(name);
        let short = short.rsplit('\\').next().unwrap_or(short);
        let pattern = format!(
            r"^\s*(?:(?:abstract|final|readonly)\s+)*(?:class|trait|interface|enum)\s+{}\b",
            regex::escape(short)
        );
        let cut = self.truncated.get();
        let declared = self.project_definitions(kind, from, short, &pattern);
        self.truncated.set(cut);
        let namespaced = |t: &str, line: usize| {
            let ns = search::php_namespace(search::php_block(t, line)).unwrap_or_default();
            match ns {
                "" => short.to_owned(),
                _ => format!("{ns}\\{short}"),
            }
        };
        let matching = declared
            .into_iter()
            .filter_map(|h| {
                let t = self.text_of(&h.path)?;
                let code = search::literal_lines(kind, &t).get(h.line1 - 1) != Some(&true);
                let fits = full
                    .as_ref()
                    .is_none_or(|f| namespaced(&t, h.line1 - 1) == *f);
                (code && fits).then_some(PhpClass {
                    path: h.path,
                    text: t,
                    header0: h.line1 - 1,
                })
            })
            .collect();
        (full.clone(), matching)
    }

    pub(super) fn php_typed(
        &mut self,
        here: &Path,
        text: &str,
        chain: &[String],
        word: &str,
        access: search::PhpAccess,
    ) -> Option<Vec<Candidate>> {
        let (head, fields) = chain.split_first().filter(|_| chain.len() <= 6)?;
        let (mut class, mut links, rest) = if head.as_str() == "this" {
            let (field, rest) = fields.split_first()?;
            let own = Self::php_self(here, text, self.line)?;
            let class = match self.php_property_link(&own, field) {
                Link::Class(class) => class,
                Link::Outside(full) if rest.is_empty() => {
                    let short = full.rsplit('\\').next().unwrap_or(&full);
                    let link = format!("$this->{field}: {short}");
                    return self.php_outside_answer(&full, word, access, link);
                }
                Link::Outside(_) | Link::Unread => return None,
            };
            let link = format!("$this->{field}: {}", Self::php_class_name(&class)?);
            (class, vec![link], rest)
        } else {
            let Some(ProvenClass { class, link }) =
                self.php_variable_type(here, text, head, self.line, true)
            else {
                return fields
                    .is_empty()
                    .then(|| self.php_outside_receiver(here, text, head, word, access))
                    .flatten();
            };
            (class, vec![link], fields)
        };
        for (i, field) in rest.iter().enumerate() {
            class = match self.php_property_link(&class, field) {
                Link::Class(class) => class,
                Link::Outside(full) if i + 1 == rest.len() => {
                    let short = full.rsplit('\\').next().unwrap_or(&full);
                    links.push(format!("{field}: {short}"));
                    return self.php_outside_answer(&full, word, access, links.join(" \u{2192} "));
                }
                Link::Outside(_) | Link::Unread => return None,
            };
            links.push(format!("{field}: {}", Self::php_class_name(&class)?));
        }
        let re = Regex::new(&search::php_access_patterns(word, access).join("|")).ok()?;
        let Walk::Found(hits) = self.php_walk(
            &class.path,
            &class.text,
            class.header0,
            &re,
            0,
            &mut Vec::new(),
        ) else {
            return None;
        };
        let reason = Reason::Receiver(links.join(" \u{2192} "));
        Some(
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
        )
    }

    fn php_outside_receiver(
        &mut self,
        here: &Path,
        text: &str,
        name: &str,
        word: &str,
        access: search::PhpAccess,
    ) -> Option<Vec<Candidate>> {
        let lines: Vec<&str> = text.lines().collect();
        let mut written: Option<String> = None;
        for b in search::php_variable(text, self.line + 1, name)? {
            let at = b - 1;
            let statement = lines[at..lines.len().min(at + 20)].join("\n");
            if search::php_captures(lines[at], name)
                || search::php_param_tag(&lines, at, name).is_some()
            {
                return None;
            }
            let (search::PhpBinding::Type(t) | search::PhpBinding::New(t)) =
                search::php_binding(&statement, name)?
            else {
                return None;
            };
            if *written.get_or_insert_with(|| t.clone()) != t {
                return None;
            }
        }
        let written = written?;
        let full = self.php_outside_type(here, text, self.line, &written)?;
        let short = full.rsplit('\\').next().unwrap_or(&full).to_owned();
        self.php_outside_answer(&full, word, access, format!("${name}: {short}"))
    }

    fn php_outside_type(
        &self,
        path: &Path,
        text: &str,
        line0: usize,
        written: &str,
    ) -> Option<String> {
        let block = search::php_block(text, line0);
        let search::PhpResolved { full, imported } = search::php_resolve(block, written)?;
        let map = search::php_psr4(&self.root, path.parent().unwrap_or_else(|| Path::new("")));
        let outside = matches!(
            search::php_psr4_file(&map, &full, |f| self.files.iter().any(|p| p == f)),
            search::Psr4File::OutsideProject
        );
        let proven = (imported || written.starts_with('\\')) && !map.is_empty() && outside;
        (proven && self.php_class(path, block, written).1.is_empty()).then_some(full)
    }

    fn php_outside_answer(
        &mut self,
        full: &str,
        word: &str,
        access: search::PhpAccess,
        links: String,
    ) -> Option<Vec<Candidate>> {
        let short = full.rsplit('\\').next().unwrap_or(full);
        let re = Regex::new(&search::php_access_patterns(word, access).join("|")).ok()?;
        let walk = self.php_outside_walk(Some(full), short, &re, 0, &mut Vec::new());
        let reason = Reason::Receiver(links);
        Some(match walk {
            Walk::Found(hits) => hits
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
            Walk::Nowhere | Walk::NowhereOutside | Walk::Unread => Vec::new(),
        })
    }

    fn php_class_name(class: &PhpClass) -> Option<String> {
        let l = class.text.lines().nth(class.header0)?;
        search::php_class_header(l).map(|h| h.name.to_owned())
    }

    fn php_self(path: &Path, text: &str, line0: usize) -> Option<PhpClass> {
        let lines: Vec<&str> = text.lines().collect();
        let class = search::php_enclosing_class(&lines, line0)?;
        let header = search::php_class_header(lines[class])?;
        (header.keyword != "trait").then(|| PhpClass {
            path: path.to_path_buf(),
            text: text.to_owned(),
            header0: class,
        })
    }

    fn php_type(&self, path: &Path, text: &str, line0: usize, written: &str) -> Option<PhpClass> {
        if written.eq_ignore_ascii_case("self") {
            return Self::php_self(path, text, line0);
        }
        let (_, mut matching) = self.php_class(path, search::php_block(text, line0), written);
        (matching.len() == 1).then(|| matching.remove(0))
    }

    fn php_variable_type(
        &mut self,
        here: &Path,
        text: &str,
        name: &str,
        line0: usize,
        cursor: bool,
    ) -> Option<ProvenClass> {
        let lines: Vec<&str> = text.lines().collect();
        let mut found: Option<ProvenClass> = None;
        let bound = search::php_variable(text, line0 + 1, name)?;
        let to = if cursor { line0 } else { line0 + 1 };
        let bound = match bound.iter().rposition(|&b| b <= to) {
            Some(i) if search::php_last_binding(text, bound[i] - 1, to, name) => {
                bound[i..].to_vec()
            }
            _ => bound,
        };
        for b in bound {
            let at = b - 1;
            let statement = lines[at..lines.len().min(at + 20)].join("\n");
            let binding = if search::php_captures(lines[at], name) {
                search::PhpBinding::Captured
            } else {
                search::php_binding(&statement, name)?
            };
            if cursor
                && at == line0
                && !matches!(
                    binding,
                    search::PhpBinding::Type(_) | search::PhpBinding::Captured
                )
            {
                continue;
            }
            let (class, call) = match binding {
                search::PhpBinding::Captured => {
                    let ProvenClass { class, link } =
                        self.php_variable_type(here, text, name, at.checked_sub(1)?, false)?;
                    (class, Some(link))
                }
                search::PhpBinding::ThisProperty(field) => {
                    let own = Self::php_self(here, text, at)?;
                    let class = self.php_property_type(&own, &field)?;
                    let link = format!("$this->{field}: {}", Self::php_class_name(&class)?);
                    (class, Some(link))
                }
                search::PhpBinding::Type(t) | search::PhpBinding::New(t) => {
                    (self.php_type(here, text, at, &t)?, None)
                }
                search::PhpBinding::StaticCall { class: on, method } => {
                    let class = self.php_type(here, text, at, &on)?;
                    let Returned { class, written } = self.php_returns(&class, &method)?;
                    (class, Some(format!("{on}::{method}(): {written}")))
                }
                search::PhpBinding::VarCall { var, method } => {
                    let ProvenClass { class, .. } =
                        self.php_variable_type(here, text, &var, at.checked_sub(1)?, false)?;
                    let Returned { class, written } = self.php_returns(&class, &method)?;
                    (class, Some(format!("${var}->{method}(): {written}")))
                }
                search::PhpBinding::ThisCall(method) => {
                    let class = Self::php_self(here, text, at)?;
                    let Returned { class, written } = self.php_returns(&class, &method)?;
                    (class, Some(format!("$this->{method}(): {written}")))
                }
                search::PhpBinding::Anonymous => {
                    let class = PhpClass {
                        path: here.to_path_buf(),
                        text: text.to_owned(),
                        header0: at,
                    };
                    (class, Some(format!("${name}: new class")))
                }
                search::PhpBinding::FunctionCall(f) => {
                    let Returned { class, written } = self.php_function_returns(here, &f)?;
                    (class, Some(format!("{f}(): {written}")))
                }
            };
            match &found {
                Some(one)
                    if (&one.class.path, one.class.header0) != (&class.path, class.header0) =>
                {
                    return None;
                }
                Some(_) => {}
                None => {
                    let link = match call {
                        Some(call) => call,
                        None => format!("${name}: {}", Self::php_class_name(&class)?),
                    };
                    found = Some(ProvenClass { class, link });
                }
            }
        }
        found
    }

    fn php_one_method(&mut self, class: &PhpClass, method: &str) -> Option<Hit> {
        let re =
            Regex::new(&search::php_access_patterns(method, search::PhpAccess::Call).join("|"))
                .ok()?;
        let Walk::Found(hits) = self.php_walk(
            &class.path,
            &class.text,
            class.header0,
            &re,
            0,
            &mut Vec::new(),
        ) else {
            return None;
        };
        let [hit] = <[Hit; 1]>::try_from(hits).ok()?;
        Some(hit)
    }

    fn php_returns(&mut self, class: &PhpClass, method: &str) -> Option<Returned> {
        let hit = self.php_one_method(class, method)?;
        if self.php_returns_static(&hit) {
            return Some(Returned {
                class: class.clone(),
                written: "static".to_owned(),
            });
        }
        self.php_declared_return(&hit.path, hit.line1 - 1)
    }

    fn php_returns_static(&self, hit: &Hit) -> bool {
        let Some(text) = self.text_of(&hit.path) else {
            return false;
        };
        let lines: Vec<&str> = text.lines().collect();
        let at = hit.line1 - 1;
        let Some(header) = lines.get(at..lines.len().min(at + 20)) else {
            return false;
        };
        !header
            .first()
            .is_some_and(|l| l.trim_start().starts_with('*'))
            && search::php_return_written(&header.join("\n"))
                .is_some_and(|w| w.eq_ignore_ascii_case("static"))
    }

    fn php_called(
        &mut self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
        access: search::PhpAccess,
    ) -> Option<Vec<Candidate>> {
        let (head, calls) = search::php_call_chain(before)?;
        let (mut class, mut links) = match head {
            search::PhpCallHead::This => (Self::php_self(here, text, self.line)?, Vec::new()),
            search::PhpCallHead::New(written) => (
                self.php_type(here, text, self.line, &written)?,
                vec![format!("new {written}")],
            ),
            search::PhpCallHead::Var(name) => {
                let ProvenClass { class, link } =
                    self.php_variable_type(here, text, &name, self.line, true)?;
                (class, vec![link])
            }
            search::PhpCallHead::Callable(name) => {
                let [b] = search::php_variable(text, self.line + 1, &name)?[..] else {
                    return None;
                };
                let at = b.checked_sub(1).filter(|&at| at < self.line)?;
                let line = text.lines().nth(at)?;
                let (var, method) = search::php_first_class_callable(line, &name)?;
                let class = match var.as_str() {
                    "this" => Self::php_self(here, text, at)?,
                    _ => {
                        self.php_variable_type(here, text, &var, at.checked_sub(1)?, false)?
                            .class
                    }
                };
                let Returned { class, written } = self.php_returns(&class, &method)?;
                (
                    class,
                    vec![format!("${name} = ${var}->{method}(...): {written}")],
                )
            }
        };
        for method in &calls {
            let returned = self.php_returns(&class, method)?;
            links.push(format!("{method}(): {}", returned.written));
            class = returned.class;
        }
        let re = Regex::new(&search::php_access_patterns(word, access).join("|")).ok()?;
        let Walk::Found(hits) = self.php_walk(
            &class.path,
            &class.text,
            class.header0,
            &re,
            0,
            &mut Vec::new(),
        ) else {
            return None;
        };
        let reason = Reason::Receiver(links.join(" \u{2192} "));
        Some(
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
        )
    }

    fn php_function_returns(&self, here: &Path, name: &str) -> Option<Returned> {
        let short = name.rsplit('\\').next().unwrap_or(name);
        let pattern = format!(r"^function\s+&?\s*{}\s*\(", regex::escape(short));
        let cut = self.truncated.get();
        let hits = self.project_definitions(Kind::Php, here, short, &pattern);
        self.truncated.set(cut);
        let [hit] = hits.as_slice() else {
            return None;
        };
        self.php_declared_return(&hit.path, hit.line1 - 1)
    }

    fn php_declared_return(&self, path: &Path, line0: usize) -> Option<Returned> {
        let text = self.text_of(path)?;
        let lines: Vec<&str> = text.lines().collect();
        if lines.get(line0)?.trim_start().starts_with('*') {
            return None;
        }
        let header = lines[line0..lines.len().min(line0 + 20)].join("\n");
        let written = search::php_return_type(&header)?;
        let class = self.php_type(path, &text, line0, &written)?;
        Some(Returned { class, written })
    }

    fn php_property_type(&mut self, class: &PhpClass, field: &str) -> Option<PhpClass> {
        match self.php_property_link(class, field) {
            Link::Class(class) => Some(class),
            Link::Outside(_) | Link::Unread => None,
        }
    }

    fn php_property_link(&mut self, class: &PhpClass, field: &str) -> Link {
        let Some((path, text, line0, written)) = self.php_property_written(class, field) else {
            return Link::Unread;
        };
        if let Some(class) = self.php_type(&path, &text, line0, &written) {
            return Link::Class(class);
        }
        let lines: Vec<&str> = text.lines().collect();
        if let Some(tag) = search::php_param_tag(&lines, line0, field) {
            return search::php_one_class(&tag)
                .and_then(|t| self.php_type(&path, &text, line0, &t))
                .map_or(Link::Unread, Link::Class);
        }
        self.php_outside_type(&path, &text, line0, &written)
            .map_or(Link::Unread, Link::Outside)
    }

    fn php_property_written(
        &mut self,
        class: &PhpClass,
        field: &str,
    ) -> Option<(PathBuf, String, usize, String)> {
        let re =
            Regex::new(&search::php_access_patterns(field, search::PhpAccess::Property).join("|"))
                .ok()?;
        let Walk::Found(hits) = self.php_walk(
            &class.path,
            &class.text,
            class.header0,
            &re,
            0,
            &mut Vec::new(),
        ) else {
            return None;
        };
        let [hit] = hits.as_slice() else {
            return None;
        };
        let written = if hit.text.trim_start().starts_with('*') {
            search::php_tag_type(&hit.text, field)?
        } else {
            search::php_written_type(&hit.text, field)?
        };
        let text = self.text_of(&hit.path)?;
        Some((hit.path.clone(), text, hit.line1 - 1, written))
    }

    fn php_outside_classes(&mut self, full: Option<&str>, short: &str) -> Vec<PhpClass> {
        let file = format!("{short}.php");
        self.external_files(Kind::Php)
            .iter()
            .filter(|p| p.file_name().is_some_and(|n| n == file.as_str()))
            .filter_map(|path| {
                let text = std::fs::read_to_string(path).ok()?;
                let ns = search::php_namespace(&text).unwrap_or_default();
                let named = match ns {
                    "" => short.to_owned(),
                    _ => format!("{ns}\\{short}"),
                };
                if full.is_some_and(|f| f != named) {
                    return None;
                }
                let header0 = text
                    .lines()
                    .position(|l| search::php_class_header(l).is_some_and(|h| h.name == short))?;
                Some(PhpClass {
                    path: path.clone(),
                    text,
                    header0,
                })
            })
            .collect()
    }

    fn php_outside(&mut self, full: Option<&str>, short: &str, re: &Regex) -> Walk {
        let mut hits = Vec::new();
        for class in self.php_outside_classes(full, short) {
            let lines: Vec<&str> = class.text.lines().collect();
            hits.extend(
                search::php_class_members(&lines, class.header0, re)
                    .into_iter()
                    .map(|i| Hit {
                        deleted: None,
                        path: class.path.clone(),
                        line1: i + 1,
                        byte_col: None,
                        text: lines[i].to_owned(),
                    }),
            );
        }
        if hits.is_empty() {
            Walk::NowhereOutside
        } else {
            Walk::Found(hits)
        }
    }
}
