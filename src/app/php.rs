//! `d` in PHP on `$this->name`, `self::name`, `static::name` and `parent::name`: a member of the
//! class the cursor is in, of a trait it uses or of a class it extends (#356).

use super::*;

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
        let short = name.rsplit('\\').next().unwrap_or(name);
        let (full, mut matching) = self.php_class(from, text, name);
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
    /// it: its `use` import, a name spelled from the root, else every one of that name (the
    /// file's namespace is #351). Each is its file, the file's text and its 0-based line; the
    /// full name the file gives it comes first, when it gives one.
    fn php_class(
        &mut self,
        from: &Path,
        text: &str,
        name: &str,
    ) -> (Option<String>, Vec<(PathBuf, String, usize)>) {
        let kind = Kind::Php;
        let short = name.rsplit('\\').next().unwrap_or(name);
        let imports = search::imports(kind, text);
        let full = match name.strip_prefix('\\') {
            Some(full) => Some(full.to_owned()),
            None if !name.contains('\\') => bound(&imports, short).map(|p| p.join("\\")),
            None => None,
        };
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
                let fits = full.as_ref().is_none_or(|f| namespaced(&t) == *f);
                fits.then_some((h.path, t, h.line - 1))
            })
            .collect();
        (full, matching)
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
    /// `short.php`, narrowed to the one declaring the namespace of `full` when an import names
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
