//! `d` in HTML, CSS, SCSS and Less, and on a class or an id attribute of a JSX, Vue, Svelte or
//! Astro template (#415).

use super::definition::resolution;
use super::links::in_project;
use super::*;
use search::{Attr, Sheet};

impl App {
    /// The class attribute under the cursor and its range: in an HTML file, a Vue, Svelte or
    /// Astro template, and a JSX attribute of a JavaScript or TypeScript file. Other kinds have
    /// no markup to read.
    fn attr_here(&self, kind: Option<Kind>) -> Option<(Attr, std::ops::Range<usize>)> {
        let markup = kind == Some(Kind::Html)
            || (kind.is_none()
                && (self.buf.path.as_deref())
                    .and_then(Path::extension)
                    .is_some_and(|e| e == "vue" || e == "svelte" || e == "astro"));
        if !markup && kind != Some(Kind::TsJs) {
            return None;
        }
        search::attr_at(self.line_str(), self.col, !markup)
    }

    /// The word `u` reads where `d` reads one of its own (#415): a class or an id of an
    /// attribute, `-` and all, and a custom property with its `--`.
    pub(super) fn css_word(&self) -> Option<String> {
        let kind = self.kind();
        if let Some((Attr::Class | Attr::Id, range)) = self.attr_here(kind) {
            return Some(self.line_str()[range].to_owned());
        }
        if !matches!(kind, Some(Kind::Css | Kind::Html)) {
            return None;
        }
        let (range, word) = search::word_at(self.line_str(), self.col, "-")?;
        self.line_str()[..range.start]
            .ends_with("--")
            .then(|| format!("--{word}"))
    }

    /// `d` where #415 answers: a class or an id attribute in any file with markup, and anything
    /// in a stylesheet or an HTML file. False leaves the cursor to the kind's own rules.
    pub(super) fn css_definition(&mut self, kind: Option<Kind>, here: &Path) -> bool {
        let line = self.line_str().to_owned();
        match self.attr_here(kind) {
            Some((Attr::Class, r)) => {
                self.styled_by(here, &line[r], false);
                return true;
            }
            Some((Attr::Id, r)) => {
                self.styled_by(here, &line[r], true);
                return true;
            }
            Some((Attr::Href | Attr::Src, r)) if kind == Some(Kind::Html) => {
                self.follow_html_path(here, &line[r]);
                return true;
            }
            _ => {}
        }
        match kind {
            Some(Kind::Css) => {}
            // Only a `<style>` block of an HTML file is a stylesheet; an inline `<script>` is not
            // read as JavaScript.
            Some(Kind::Html) => {
                let styles = search::style_blocks(&self.buf.lines.join("\n"));
                if styles
                    .lines()
                    .nth(self.line)
                    .is_none_or(|l| l.trim().is_empty())
                {
                    self.no_definition(&line);
                    return true;
                }
            }
            _ => return false,
        }
        let less = here.extension().is_some_and(|e| e == "less");
        match search::sheet_at(&line, self.col, less) {
            None => self.no_definition(&line),
            Some(Sheet::Import(module)) => self.follow_sheet_import(here, &module),
            Some(Sheet::Class(name)) => self.styled_by(here, &name, false),
            Some(Sheet::Id(name)) => self.styled_by(here, &name, true),
            Some(sheet) => self.sheet_declarations(here, &sheet),
        }
        true
    }

    /// "no definition for" the word under the cursor, read with `-`.
    fn no_definition(&mut self, line: &str) {
        self.message = match search::word_at(line, self.col, "-") {
            Some((_, word)) => resolution(word, None, &[], None, false),
            None => "no word".into(),
        };
    }

    /// The rules that style the class (or the id) `name`, in the project's stylesheets and the
    /// `<style>` blocks of its markup, never outside the project.
    fn styled_by(&mut self, here: &Path, name: &str, id: bool) {
        // A file that composes the name with `&` holds its last piece at least. Every file is
        // read, with no grep and so no cap on its hits: Bootstrap writes `btn` thousands of times.
        let piece = name
            .rsplit(['-', '_'])
            .find(|p| !p.is_empty())
            .unwrap_or(name);
        let files: Vec<PathBuf> = (self.files.iter())
            .filter(|p| search::styles_in(p).is_some())
            .cloned()
            .collect();
        let mut found: Vec<Candidate> = Vec::new();
        for path in files {
            let Some(text) = self.text_of(&path).filter(|t| t.contains(piece)) else {
                continue;
            };
            let styles = match search::styles_in(&path) {
                Some(true) => text.clone(),
                _ => search::style_blocks(&text),
            };
            let lines: Vec<&str> = text.lines().collect();
            for rule in search::rules(&styles) {
                let names = if id { &rule.ids } else { &rule.classes };
                let fresh = found
                    .last()
                    .is_none_or(|c| c.hit.path != path || c.hit.line != rule.line);
                if fresh && names.iter().any(|n| n == name) {
                    found.push(Candidate {
                        hit: Hit {
                            deleted: None,
                            path: path.clone(),
                            line: rule.line,
                            col: 0,
                            text: lines
                                .get(rule.line - 1)
                                .copied()
                                .unwrap_or_default()
                                .to_owned(),
                        },
                        reason: Reason::ByName,
                    });
                }
            }
        }
        self.truncated.set(false);
        self.show_definitions(Kind::Css, name, here, found, None);
    }

    /// The declarations of a stylesheet's name: a namespaced one in the module its `@use` binds
    /// (`via import`), else by name in the files that can declare it.
    fn sheet_declarations(&mut self, here: &Path, sheet: &Sheet) {
        let (word, ns) = match sheet {
            Sheet::Var(ns, n) | Sheet::Mixin(ns, n) | Sheet::Function(ns, n) => (n, ns.as_deref()),
            Sheet::Custom(n) | Sheet::LessVar(n) | Sheet::Placeholder(n) | Sheet::Keyframes(n) => {
                (n, None)
            }
            Sheet::Class(n) | Sheet::Id(n) | Sheet::Import(n) => (n, None),
        };
        let Ok(re) = Regex::new(&search::sheet_patterns(sheet).join("|")) else {
            return;
        };
        let module = ns.and_then(|ns| {
            let uses = search::sass_uses(&self.buf.lines.join("\n"));
            let (_, module) = uses.into_iter().find(|(n, _)| n.as_deref() == Some(ns))?;
            self.sheet_module(here, &module, false)
        });
        let mut found: Vec<Candidate> = module
            .iter()
            .flat_map(|path| {
                let reason = Reason::Import(path.display().to_string());
                self.matching_lines(path, &re, false)
                    .into_iter()
                    .map(move |hit| Candidate {
                        hit,
                        reason: reason.clone(),
                    })
            })
            .collect();
        if found.is_empty() {
            let wanted = |p: &Path| {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or_default();
                match sheet {
                    Sheet::LessVar(_) => ext == "less",
                    Sheet::Var(..)
                    | Sheet::Mixin(..)
                    | Sheet::Function(..)
                    | Sheet::Placeholder(_) => ext == "scss" || ext == "sass",
                    _ => search::styles_in(p).is_some(),
                }
            };
            found = (self.files.iter())
                .filter(|p| wanted(p))
                .flat_map(|p| self.matching_lines(p, &re, search::styles_in(p) == Some(false)))
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect();
        }
        self.truncated.set(false);
        self.show_definitions(Kind::Css, word, here, found, None);
    }

    /// The lines of `path`, from the root, that `re` matches; in a
    /// `<style>` block only, when `blocks`.
    fn matching_lines(&self, path: &Path, re: &Regex, blocks: bool) -> Vec<Hit> {
        let Some(text) = self.text_of(path) else {
            return Vec::new();
        };
        let styles = match blocks {
            true => search::style_blocks(&text),
            false => text.clone(),
        };
        styles
            .lines()
            .zip(text.lines())
            .enumerate()
            .filter(|(_, (s, _))| re.is_match(&search::css_code(s)))
            .map(|(i, (_, t))| Hit {
                deleted: None,
                path: path.to_path_buf(),
                line: i + 1,
                col: 0,
                text: t.to_owned(),
            })
            .collect()
    }

    /// The file a stylesheet's import of `module` is: beside the importing file `here`, then in
    /// the `node_modules` of its directories up to the root, as a package. Sass tries its
    /// partials and index files, Less adds `.less`; `css` reads a plain CSS `@import`.
    fn sheet_module(&self, here: &Path, module: &str, css: bool) -> Option<PathBuf> {
        let ext = here
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        let candidates = match ext {
            "scss" | "sass" if !css => search::sass_candidates(module),
            _ => search::import_candidates(module, ext == "less"),
        };
        let dir = here.parent().unwrap_or(Path::new(""));
        let local = !module.starts_with('~');
        let beside = candidates
            .iter()
            .filter(|_| local)
            .filter_map(|c| in_project(dir, &c.to_string_lossy()))
            .find(|rel| self.root.join(rel).is_file());
        beside.or_else(|| {
            dir.ancestors().find_map(|d| {
                let modules = d.join("node_modules");
                candidates
                    .iter()
                    .map(|c| modules.join(c))
                    .find(|p| self.root.join(p).is_file())
            })
        })
    }

    /// `d` on the path of an `@import`, a `@use` or a `@forward`: the file it names opens.
    fn follow_sheet_import(&mut self, here: &Path, module: &str) {
        if search::is_url(module) && !module.starts_with("sass:") {
            self.message = "no file in the project: a URL".into();
            return;
        }
        if module.starts_with("sass:") {
            self.message = format!("{module}: builtin, no source");
            return;
        }
        match self.sheet_module(here, module, false) {
            Some(path) => {
                let status = format!("path {}", path.display());
                self.open_link(&self.root.join(&path), 1, status);
            }
            None => self.message = format!("no file {module}"),
        }
    }

    /// `d` on the `href` or `src` of an HTML element: the file it names, relative to `here`
    /// (one the project holds whose path ends with it after a leading `/`), at the element
    /// whose `id` or `name` its `#fragment` is.
    fn follow_html_path(&mut self, here: &Path, value: &str) {
        if search::is_url(value) {
            self.message = "no file in the project: a URL".into();
            return;
        }
        let (path, fragment) = match value.split_once('#') {
            Some((p, f)) => (p, Some(f)),
            None => (value, None),
        };
        let path = path.split(['?']).next().unwrap_or(path);
        let dir = here.parent().unwrap_or(Path::new(""));
        let files: Vec<PathBuf> = match path {
            "" => vec![here.to_path_buf()],
            p if p.starts_with('/') => {
                let tail = Path::new(p.trim_start_matches('/'));
                self.files
                    .iter()
                    .filter(|f| f.ends_with(tail))
                    .cloned()
                    .collect()
            }
            p => in_project(dir, p)
                .filter(|rel| self.root.join(rel).is_file())
                .into_iter()
                .collect(),
        };
        let [file] = files.as_slice() else {
            match files.is_empty() {
                true => self.message = format!("no file {path}"),
                false => {
                    let found = self.module_candidates(files);
                    self.show_definitions(Kind::Html, path, here, found, None);
                }
            }
            return;
        };
        let line = match fragment.filter(|f| !f.is_empty()) {
            None => 1,
            Some(f) => match self
                .text_of(file)
                .and_then(|t| search::element_with_id(&t, f))
            {
                Some(line) => line,
                None => {
                    self.message = format!("no id #{f} in {}", file.display());
                    return;
                }
            },
        };
        let shown = match path {
            "" => String::new(),
            _ => file.display().to_string(),
        };
        let anchor = fragment.map_or(String::new(), |f| format!("#{f}"));
        self.open_link(&self.root.join(file), line, format!("path {shown}{anchor}"));
    }
}
