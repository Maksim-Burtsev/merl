//! `d` in PHP on `$this->name`, `self::name`, `static::name` and `parent::name`: a member of the
//! class the cursor is in, of a trait it uses or of a class it extends (#356).

use super::*;
use std::ops::Range;

/// A class the project declares: its file, the file's text and the 0-based line of its header.
type PhpClass = (PathBuf, String, usize);

/// Where the walk of [`App::php_link`] reads a member from.
enum Walk {
    /// Declared here.
    Found(Vec<Hit>),
    /// Nowhere on the walk, which went to its end: `true` when it ended in a class outside the
    /// project, which may inherit more than the walk read.
    Nowhere(bool),
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

    /// What `d` answers on a class name, or on `Class::word`, at `range` of the cursor's line
    /// (#351), in a project whose `composer.json` maps namespaces to directories (PSR-4): the
    /// name resolved as PHP resolves it ([`search::php_resolve`]), and the file the map puts it
    /// in. When that file declares the class, the answer is its header, or `word` looked up in
    /// it as [`Self::php_walk`] does. A name the map does not cover is outside the project: the
    /// class in `vendor/`, in its own namespace, is read first. `None` leaves the word to the
    /// other rules: no map, a name a group `use` binds, a mapped file missing or declaring
    /// something else, a member found nowhere on the walk, a name outside whose class `vendor/`
    /// does not declare it in.
    fn php_named(
        &mut self,
        here: &Path,
        text: &str,
        range: Range<usize>,
        access: search::PhpAccess,
    ) -> Option<Vec<Candidate>> {
        let line = self.line_str();
        let word = line[range.clone()].to_owned();
        let (written, member) = search::php_class_at(line, range)?;
        let map = search::php_psr4(&self.root, here.parent().unwrap_or(Path::new("")));
        if map.is_empty() {
            return None;
        }
        let (full, used) = search::php_resolve(text, &written)?;
        let (ns, short) = match full.rsplit_once('\\') {
            Some((ns, short)) => (ns, short),
            None => ("", full.as_str()),
        };
        let path = match search::php_psr4_file(&map, &full, |f| self.files.iter().any(|p| p == f)) {
            Some(Ok(path)) => path,
            Some(Err(())) => return None,
            // Outside the project: what `vendor/` declares under that path, before any namesake.
            None => {
                let imports = [(
                    short.to_owned(),
                    full.split('\\').map(str::to_owned).collect(),
                )];
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
                && search::php_class_header(lines[i]).is_some_and(|(_, n, _)| n == short)
        })?;
        if search::php_namespace(&t).unwrap_or_default() != ns {
            return None;
        }
        let reason = match used {
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
                    Walk::Nowhere(_) | Walk::Unread => return None,
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

    /// What `d` answers for `word` behind `link` (`this`, `self`, `static` or `parent`, written
    /// `$this->` or `link::`) on the cursor's line of `text`: `None` leaves the word to the search
    /// by name, as for a link the rules cannot read, a cursor in an anonymous class or in a
    /// trait's own body, whose `$this` is whichever class uses it.
    ///
    /// The class around the cursor declares the member itself, else a trait its body uses, else
    /// the class it extends, walked the same way up to eight levels, as [`Self::hierarchy`] is.
    /// A class the project does not declare is read from its file in `vendor/`, the one its
    /// import names when it has one, and a member found nowhere there is no definition: never
    /// another class's namesake. A walk that ends inside the project with nothing found leaves
    /// the word to the search by name, which also knows an interface's constants.
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
        let (keyword, name, parent) = search::php_class_header(lines[class])?;
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
                    self.php_walk_class(here, text, parent, &re, 1, &mut Vec::new()),
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
            Walk::Nowhere(true) => Some(Vec::new()),
            Walk::Nowhere(false) | Walk::Unread => None,
        }
    }

    /// The members `re` matches of the class on 0-based line `class` of `text`, the file `path`,
    /// then of its traits, then of the class it extends.
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
        // A trait outside the project that does not declare it says nothing of the parent.
        for t in search::php_class_traits(&lines, class) {
            match self.php_walk_class(path, text, &t, re, depth + 1, seen) {
                Walk::Nowhere(_) => {}
                other => return other,
            }
        }
        match search::php_class_header(lines[class]).and_then(|(_, _, parent)| parent) {
            Some(parent) => self.php_walk_class(path, text, parent, re, depth + 1, seen),
            None => Walk::Nowhere(false),
        }
    }

    /// [`Self::php_walk`] from the class `name`, as the file `from` (its `text`) writes it
    /// ([`Self::php_class`]). A class the project does not declare is read in `vendor/`.
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
                let (path, t, line) = matching.remove(0);
                self.php_walk(&path, &t, line, re, depth, seen)
            }
            0 => self.php_outside(full.as_deref(), short, re),
            _ => Walk::Unread,
        }
    }

    /// The project's declarations of the class `name` as the file `from` (its `text`) writes
    /// it, resolved as PHP resolves it ([`search::php_resolve`], #351), else, for a name a group
    /// `use` binds, every one of that name. Each is its file, the file's text and its 0-based
    /// line; the full name comes first, when the file resolves it.
    fn php_class(
        &mut self,
        from: &Path,
        text: &str,
        name: &str,
    ) -> (Option<String>, Vec<(PathBuf, String, usize)>) {
        let kind = Kind::Php;
        let full = search::php_resolve(text, name).map(|(full, _)| full);
        let short = full.as_deref().unwrap_or(name);
        let short = short.rsplit('\\').next().unwrap_or(short);
        let pattern = format!(
            r"^\s*(?:(?:abstract|final|readonly)\s+)*(?:class|trait|interface|enum)\s+{}\b",
            regex::escape(short)
        );
        let cut = self.truncated.get();
        let declared = self.project_definitions(kind, from, short, &pattern);
        self.truncated.set(cut);
        let namespaced = |t: &str| {
            let ns = search::php_namespace(t).unwrap_or_default();
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
                let fits = full.as_ref().is_none_or(|f| namespaced(&t) == *f);
                (code && fits).then_some((h.path, t, h.line - 1))
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
                let (class, link) = self.php_variable_type(here, text, head)?;
                (class, vec![link], fields)
            }
        };
        for field in rest {
            class = self.php_property_type(&class, field)?;
            links.push(format!("{field}: {}", self.php_class_name(&class)?));
        }
        let re = Regex::new(&search::php_access_patterns(word, access).join("|")).ok()?;
        let (path, t, line) = class;
        let Walk::Found(hits) = self.php_walk(&path, &t, line, &re, 0, &mut Vec::new()) else {
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

    /// The name of the class on 0-based `line` of `text`.
    fn php_class_name(&self, (_, text, line): &PhpClass) -> Option<String> {
        let l = text.lines().nth(*line)?;
        search::php_class_header(l).map(|(_, name, _)| name.to_owned())
    }

    /// The class 0-based line `at` of `text`, the file `path`, stands in: `$this` and `self`
    /// there. Not a trait, whose `$this` is whichever class uses it, nor an anonymous class.
    fn php_self(&self, path: &Path, text: &str, at: usize) -> Option<PhpClass> {
        let lines: Vec<&str> = text.lines().collect();
        let class = search::php_enclosing_class(&lines, at)?;
        let (keyword, _, _) = search::php_class_header(lines[class])?;
        (keyword != "trait").then(|| (path.to_path_buf(), text.to_owned(), class))
    }

    /// The project's one class `written` names in the file `path`, whose 0-based line `at` writes
    /// it: `self` is the class around that line.
    fn php_type(&mut self, path: &Path, text: &str, at: usize, written: &str) -> Option<PhpClass> {
        if written.eq_ignore_ascii_case("self") {
            return self.php_self(path, text, at);
        }
        let (_, mut matching) = self.php_class(path, text, written);
        (matching.len() == 1).then(|| matching.remove(0))
    }

    /// The class every binding of `$name` above the cursor reads, in its function, with the link
    /// that proves it: `$event: UserUnsubscribed`, `$song: Song::make(): Song`. A binding on the
    /// cursor's own line other than its function's header is read after the cursor is.
    fn php_variable_type(
        &mut self,
        here: &Path,
        text: &str,
        name: &str,
    ) -> Option<(PhpClass, String)> {
        let lines: Vec<&str> = text.lines().collect();
        let mut found: Option<(PhpClass, String)> = None;
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
                search::PhpBinding::Static(on, method) => {
                    let class = self.php_type(here, text, at, &on)?;
                    let (ty, returns) = self.php_returns(class, &method)?;
                    (ty, Some(format!("{on}::{method}(): {returns}")))
                }
                search::PhpBinding::This(method) => {
                    let class = self.php_self(here, text, at)?;
                    let (ty, returns) = self.php_returns(class, &method)?;
                    (ty, Some(format!("$this->{method}(): {returns}")))
                }
                search::PhpBinding::Function(f) => {
                    let (ty, returns) = self.php_function_returns(here, &f)?;
                    (ty, Some(format!("{f}(): {returns}")))
                }
            };
            match &found {
                Some(((p, _, l), _)) if (p, *l) != (&class.0, class.2) => return None,
                Some(_) => {}
                None => {
                    let link = match call {
                        Some(call) => call,
                        None => format!("${name}: {}", self.php_class_name(&class)?),
                    };
                    found = Some((class, link));
                }
            }
        }
        found
    }

    /// The class the method `method` of `class`, walked as [`Self::php_walk`] walks it, declares
    /// it returns, and that return type as written.
    fn php_returns(&mut self, class: PhpClass, method: &str) -> Option<(PhpClass, String)> {
        let re =
            Regex::new(&search::php_access_patterns(method, search::PhpAccess::Call).join("|"))
                .ok()?;
        let (path, t, line) = class;
        let Walk::Found(hits) = self.php_walk(&path, &t, line, &re, 0, &mut Vec::new()) else {
            return None;
        };
        let [hit] = hits.as_slice() else {
            return None;
        };
        self.php_declared_return(&hit.path, hit.line - 1)
    }

    /// The class the function `name` declares it returns: the project's one `function name(`
    /// outside a class.
    fn php_function_returns(&mut self, here: &Path, name: &str) -> Option<(PhpClass, String)> {
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

    /// The class the function or method whose header starts on 0-based line `at` of the file
    /// `path` declares it returns, and the type as written; a docblock's `@method` is no header.
    fn php_declared_return(&mut self, path: &Path, at: usize) -> Option<(PhpClass, String)> {
        let text = self.text_of(path)?;
        let lines: Vec<&str> = text.lines().collect();
        if lines.get(at)?.trim_start().starts_with('*') {
            return None;
        }
        let header = lines[at..lines.len().min(at + 20)].join("\n");
        let written = search::php_return_type(&header)?;
        let class = self.php_type(path, &text, at, &written)?;
        Some((class, written))
    }

    /// The class the property `field` of `class` is typed with: its one declaration on the walk
    /// of [`Self::php_walk`], a typed property or a promoted constructor parameter. A docblock's
    /// `@property` is none.
    fn php_property_type(&mut self, class: &PhpClass, field: &str) -> Option<PhpClass> {
        let re =
            Regex::new(&search::php_access_patterns(field, search::PhpAccess::Property).join("|"))
                .ok()?;
        let (path, t, line) = class;
        let Walk::Found(hits) = self.php_walk(path, t, *line, &re, 0, &mut Vec::new()) else {
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
                .find(|&i| search::php_class_header(lines[i]).is_some_and(|(_, n, _)| n == short))
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
            true => Walk::Nowhere(true),
            false => Walk::Found(hits),
        }
    }
}
