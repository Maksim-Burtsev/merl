use super::*;
use std::ops::Range;

/// A class the project declares.
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

/// Where the walk of [`App::php_link`] reads a member from.
enum Walk {
    Found(Vec<Hit>),
    Nowhere,
    /// Nowhere on the walk, which ended in a class outside the project that may inherit more
    /// than the walk read.
    NowhereOutside,
    /// A link the rules cannot read: a trait or a parent the project declares twice with no
    /// import to choose, or one the walk has seen already.
    Unread,
}

impl App {
    /// What `d` answers in PHP before the import step: `word` behind `$this->`, `self::`,
    /// `static::` or `parent::` ([`Self::php_link`], #356; `self::$name` is a static property),
    /// behind a receiver whose class is proven ([`Self::php_typed`], #361), and a class name or
    /// its member ([`Self::php_named`], #351). `before` is the line up to the word as
    /// [`App::written`] spells it, `chain` the names in front of the word. `None` goes on to the
    /// other rules.
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
        let access = match self.line_str()[range.end..].trim_start().starts_with('(') {
            true => search::PhpAccess::Call,
            false if dotted || property.is_some() => search::PhpAccess::Property,
            false => search::PhpAccess::Constant,
        };
        if let Some(link) = link
            && let Some(found) = self.php_link(here, text, &link, word, access)
        {
            return Some(found);
        }
        // `$x->word` and `$this->f->word` on a receiver whose class is proven (#361).
        let receiver = before.strip_suffix(&format!("${}.", chain.join(".")));
        if dotted
            && !chain.is_empty()
            && receiver.is_some_and(|b| !b.ends_with(|c: char| is_word(c) || c == ':' || c == '$'))
            && let Some(found) = self.php_typed(here, text, chain, word, access)
        {
            return Some(found);
        }
        self.php_named(here, text, range, access)
    }

    fn php_named(
        &mut self,
        here: &Path,
        text: &str,
        range: Range<usize>,
        access: search::PhpAccess,
    ) -> Option<Vec<Candidate>> {
        let line = self.line_str();
        let word = line[range.clone()].to_owned();
        let search::PhpClassAt { written, member } = search::php_class_at(line, range)?;
        let map = search::php_psr4(&self.root, here.parent().unwrap_or(Path::new("")));
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
            // Outside the project: what `vendor/` declares under that path, before any namesake.
            search::Psr4File::OutsideProject => {
                let imports = [search::Import {
                    name: short.to_owned(),
                    path: full.split('\\').map(str::to_owned).collect(),
                }];
                let (word, chain) = match member {
                    true => (word, vec![short.to_owned()]),
                    false => (short.to_owned(), Vec::new()),
                };
                // Only the class itself: a hit by name, or one of another namespace, is no proof.
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
        let found = match member {
            false => vec![Hit {
                deleted: None,
                path: path.clone(),
                line: class + 1,
                col: 0,
                text: lines[class].to_owned(),
            }],
            true => {
                let re = Regex::new(&search::php_access_patterns(&word, access).join("|")).ok()?;
                match self.php_walk(&path, &t, class, &re, 0, &mut Vec::new()) {
                    Walk::Found(hits) => hits,
                    // Nothing read says it is not there: `__callStatic`, a `vendor/` not installed.
                    Walk::Nowhere | Walk::NowhereOutside | Walk::Unread => return None,
                }
            }
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
        let (reason, walk) = match link {
            "parent" => {
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
            }
            _ => {
                let reason = Reason::Receiver(format!(
                    "{}: {name}",
                    if link == "this" { "$this" } else { link }
                ));
                let mut seen = Vec::new();
                (reason, self.php_walk(here, text, class, &re, 0, &mut seen))
            }
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
                line: i + 1,
                col: 0,
                text: lines[i].to_owned(),
            })
            .collect();
        if !own.is_empty() {
            return Walk::Found(own);
        }
        let block = search::php_block(text, class);
        for t in search::php_class_traits(&lines, class) {
            match self.php_walk_class(path, block, &t, re, depth + 1, seen) {
                Walk::Nowhere | Walk::NowhereOutside => {}
                other => return other,
            }
        }
        match search::php_class_header(lines[class]).and_then(|h| h.extends) {
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
            0 => self.php_outside(full.as_deref(), short, re),
            _ => Walk::Unread,
        }
    }

    fn php_class(
        &mut self,
        from: &Path,
        text: &str,
        name: &str,
    ) -> (Option<String>, Vec<PhpClass>) {
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
                // A header inside a `/* */` block or a heredoc declares nothing (#361).
                let code = search::literal_lines(kind, &t).get(h.line - 1) != Some(&true);
                let fits = full
                    .as_ref()
                    .is_none_or(|f| namespaced(&t, h.line - 1) == *f);
                (code && fits).then_some(PhpClass {
                    path: h.path,
                    text: t,
                    header0: h.line - 1,
                })
            })
            .collect();
        (full.clone(), matching)
    }

    /// What `d` answers for `word` behind `$x->…->` on the cursor's line of `text` (#361), `chain`
    /// the names in front of it with `$` dropped: the class of `$x` proven, then each name a
    /// typed property of the class before it, up to six names, and `word` looked up in the last
    /// class as [`Self::php_walk`] does. `$x` is `$this` in a class, or a variable whose every
    /// binding in its function reads one class ([`search::php_binding`]). `None` leaves the word
    /// to the search by name: a name not proven, or a member found nowhere on the walk.
    pub(super) fn php_typed(
        &mut self,
        here: &Path,
        text: &str,
        chain: &[String],
        word: &str,
        access: search::PhpAccess,
    ) -> Option<Vec<Candidate>> {
        // ponytail: six names in front of the word, as for Python.
        let (head, fields) = chain.split_first().filter(|_| chain.len() <= 6)?;
        let (mut class, mut links, rest) = match head.as_str() {
            // `$this->word` alone is [`Self::php_link`]'s.
            "this" => {
                let (field, rest) = fields.split_first()?;
                let own = self.php_self(here, text, self.line)?;
                let class = self.php_property_type(&own, field)?;
                let link = format!("$this->{field}: {}", self.php_class_name(&class)?);
                (class, vec![link], rest)
            }
            _ => {
                let ProvenClass { class, link } = self.php_variable_type(here, text, head)?;
                (class, vec![link], fields)
            }
        };
        for field in rest {
            class = self.php_property_type(&class, field)?;
            links.push(format!("{field}: {}", self.php_class_name(&class)?));
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

    fn php_class_name(&self, class: &PhpClass) -> Option<String> {
        let l = class.text.lines().nth(class.header0)?;
        search::php_class_header(l).map(|h| h.name.to_owned())
    }

    /// The class `line0` of `text`, the file `path`, stands in: `$this` and `self` there. Not
    /// a trait, whose `$this` is whichever class uses it, nor an anonymous class.
    fn php_self(&self, path: &Path, text: &str, line0: usize) -> Option<PhpClass> {
        let lines: Vec<&str> = text.lines().collect();
        let class = search::php_enclosing_class(&lines, line0)?;
        let header = search::php_class_header(lines[class])?;
        (header.keyword != "trait").then(|| PhpClass {
            path: path.to_path_buf(),
            text: text.to_owned(),
            header0: class,
        })
    }

    fn php_type(
        &mut self,
        path: &Path,
        text: &str,
        line0: usize,
        written: &str,
    ) -> Option<PhpClass> {
        if written.eq_ignore_ascii_case("self") {
            return self.php_self(path, text, line0);
        }
        let (_, mut matching) = self.php_class(path, search::php_block(text, line0), written);
        (matching.len() == 1).then(|| matching.remove(0))
    }

    /// The class every binding of `$name` above the cursor reads, in its function: `$event: UserUnsubscribed`, `$song: Song::make(): Song`. A binding on the
    /// cursor's own line other than its function's header is read after the cursor is.
    fn php_variable_type(&mut self, here: &Path, text: &str, name: &str) -> Option<ProvenClass> {
        let lines: Vec<&str> = text.lines().collect();
        let mut found: Option<ProvenClass> = None;
        for b in search::php_variable(text, self.line + 1, name)? {
            let at = b - 1;
            // ponytail: a statement read over twenty lines.
            let statement = lines[at..lines.len().min(at + 20)].join("\n");
            let binding = search::php_binding(&statement, name)?;
            if at == self.line && !matches!(binding, search::PhpBinding::Type(_)) {
                continue;
            }
            let (class, call) = match binding {
                search::PhpBinding::Type(t) | search::PhpBinding::New(t) => {
                    (self.php_type(here, text, at, &t)?, None)
                }
                search::PhpBinding::StaticCall { class: on, method } => {
                    let class = self.php_type(here, text, at, &on)?;
                    let Returned { class, written } = self.php_returns(class, &method)?;
                    (class, Some(format!("{on}::{method}(): {written}")))
                }
                search::PhpBinding::ThisCall(method) => {
                    let class = self.php_self(here, text, at)?;
                    let Returned { class, written } = self.php_returns(class, &method)?;
                    (class, Some(format!("$this->{method}(): {written}")))
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
                        None => format!("${name}: {}", self.php_class_name(&class)?),
                    };
                    found = Some(ProvenClass { class, link });
                }
            }
        }
        found
    }

    /// The class the method `method` of `class`, walked as [`Self::php_walk`] walks it, declares
    /// it returns.
    fn php_returns(&mut self, class: PhpClass, method: &str) -> Option<Returned> {
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
        let [hit] = hits.as_slice() else {
            return None;
        };
        self.php_declared_return(&hit.path, hit.line - 1)
    }

    /// The class the function `name` declares it returns: the project's one `function name(`
    /// outside a class.
    fn php_function_returns(&mut self, here: &Path, name: &str) -> Option<Returned> {
        let short = name.rsplit('\\').next().unwrap_or(name);
        let pattern = format!(r"^function\s+&?\s*{}\s*\(", regex::escape(short));
        let cut = self.truncated.get();
        let hits = self.project_definitions(Kind::Php, here, short, &pattern);
        self.truncated.set(cut);
        let [hit] = hits.as_slice() else {
            return None;
        };
        self.php_declared_return(&hit.path, hit.line - 1)
    }

    /// The class the function or method whose header starts on `line0` of the file `path`
    /// declares it returns; a docblock's `@method` is no header.
    fn php_declared_return(&mut self, path: &Path, line0: usize) -> Option<Returned> {
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

    /// The class the property `field` of `class` is typed with: its one declaration on the walk
    /// of [`Self::php_walk`], a typed property or a promoted constructor parameter. A docblock's
    /// `@property` is none.
    fn php_property_type(&mut self, class: &PhpClass, field: &str) -> Option<PhpClass> {
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
        if hit.text.trim_start().starts_with('*') {
            return None;
        }
        let written = search::php_written_type(&hit.text, field)?;
        let text = self.text_of(&hit.path)?;
        self.php_type(&hit.path, &text, hit.line - 1, &written)
    }

    /// The members `re` matches of the class `short` outside the project, in `vendor/`: its
    /// `short.php`, narrowed to the one declaring the namespace of `full` when the file resolves
    /// it. What the class inherits there is not read.
    fn php_outside(&mut self, full: Option<&str>, short: &str, re: &Regex) -> Walk {
        let file = format!("{short}.php");
        let files: Vec<PathBuf> = self
            .external_files(Kind::Php)
            .iter()
            .filter(|p| p.file_name().is_some_and(|n| n == file.as_str()))
            .cloned()
            .collect();
        let mut hits = Vec::new();
        for path in files {
            let Ok(t) = std::fs::read_to_string(&path) else {
                continue;
            };
            let ns = search::php_namespace(&t).unwrap_or_default();
            let named = match ns {
                "" => short.to_owned(),
                _ => format!("{ns}\\{short}"),
            };
            if full.is_some_and(|f| f != named) {
                continue;
            }
            let lines: Vec<&str> = t.lines().collect();
            let Some(class) = (0..lines.len())
                .find(|&i| search::php_class_header(lines[i]).is_some_and(|h| h.name == short))
            else {
                continue;
            };
            hits.extend(
                search::php_class_members(&lines, class, re)
                    .into_iter()
                    .map(|i| Hit {
                        deleted: None,
                        path: path.clone(),
                        line: i + 1,
                        col: 0,
                        text: lines[i].to_owned(),
                    }),
            );
        }
        match hits.is_empty() {
            true => Walk::NowhereOutside,
            false => Walk::Found(hits),
        }
    }
}
