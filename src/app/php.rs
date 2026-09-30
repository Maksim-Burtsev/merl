//! `d` in PHP on `$this->name`, `self::name`, `static::name` and `parent::name`: a member of the
//! class the cursor is in, of a trait it uses or of a class it extends (#356).

use super::*;

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

    /// [`Self::php_walk`] from the class `name`, as the file `from` (its `text`) writes it: its
    /// `use` import, a name spelled from the root, else the project's one declaration of it
    /// (the file's namespace is #351). A class the project does not declare is read in
    /// `vendor/`.
    fn php_walk_class(
        &mut self,
        from: &Path,
        text: &str,
        name: &str,
        re: &Regex,
        depth: usize,
        seen: &mut Vec<(PathBuf, usize)>,
    ) -> Walk {
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
        let mut matching: Vec<(PathBuf, String, usize)> = declared
            .into_iter()
            .filter_map(|h| {
                let t = self.text_of(&h.path)?;
                let fits = full.as_ref().is_none_or(|f| namespaced(&t) == *f);
                fits.then_some((h.path, t, h.line - 1))
            })
            .collect();
        match matching.len() {
            1 => {
                let (path, t, line) = matching.remove(0);
                self.php_walk(&path, &t, line, re, depth, seen)
            }
            0 => self.php_outside(full.as_deref(), short, re),
            _ => Walk::Unread,
        }
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
