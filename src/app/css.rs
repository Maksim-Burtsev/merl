use super::links::in_project;
use super::resolution::resolution;
use super::{App, Candidate, Hit, Kind, Path, PathBuf, Reason, Regex, search};
use search::{Attr, Sheet};

impl App {
    fn attr_here(&self, kind: Option<Kind>) -> Option<(Attr, std::ops::Range<usize>)> {
        let component = (self.buf.path.as_deref())
            .and_then(Path::extension)
            .is_some_and(|e| e == "vue" || e == "svelte" || e == "astro");
        let markup = kind == Some(Kind::Html) || component;
        if !markup && kind != Some(Kind::TsJs) {
            return None;
        }
        search::attr_at(
            self.line_str(),
            self.col,
            if markup {
                search::AttrLine::Markup
            } else {
                search::AttrLine::Jsx
            },
        )
    }

    pub(super) fn css_word(&self) -> Option<String> {
        let kind = self.kind();
        if let Some((Attr::Class | Attr::Id, range)) = self.attr_here(kind) {
            return Some(self.line_str().get(range)?.to_owned());
        }
        if !matches!(kind, Some(Kind::Css | Kind::Html)) {
            return None;
        }
        let (range, word) = search::word_at(self.line_str(), self.col, "-")?;
        self.line_str()
            .get(..range.start)?
            .ends_with("--")
            .then(|| format!("--{word}"))
    }

    #[expect(
        clippy::string_slice,
        reason = "attr_here returns the range of an attribute's value on this line"
    )]
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
            Some(Kind::TsJs) => return self.css_module(here),
            _ => return false,
        }
        let syntax = search::SheetSyntax::of_extension(
            here.extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default(),
        );
        match search::sheet_at(&line, self.col, syntax) {
            None => self.no_definition(&line),
            Some(Sheet::Import(module)) => self.follow_sheet_import(here, &module),
            Some(Sheet::Class(name)) => self.styled_by(here, &name, false),
            Some(Sheet::Id(name)) => self.styled_by(here, &name, true),
            Some(sheet) => self.sheet_declarations(here, &sheet),
        }
        true
    }

    fn no_definition(&mut self, line: &str) {
        self.message = match search::word_at(line, self.col, "-") {
            Some((_, word)) => resolution(word, None, &[], None, false),
            None => "no word".into(),
        };
    }

    fn styled_by(&mut self, here: &Path, name: &str, id: bool) {
        let files: Vec<PathBuf> = (self.files.iter())
            .filter(|p| search::styles_in(p).is_some())
            .cloned()
            .collect();
        self.styled_in(here, name, id, files, &Reason::ByName);
    }

    fn css_module(&mut self, here: &Path) -> bool {
        let line = self.line_str();
        let Some((range, word)) = search::word_at(line, self.col, "") else {
            return false;
        };
        let chain = search::qualifier(line, range.start);
        let object = match (
            chain.as_slice(),
            line.get(..range.start).is_some_and(|b| b.ends_with('.')),
        ) {
            ([head], true) => Some(head.as_str()),
            ([], false) => None,
            _ => return false,
        };
        let name = object.unwrap_or(word);
        let text = self.buf.lines.join("\n");
        let code = search::script_text(here, &text, Some(self.line));
        if !search::bindings(Kind::TsJs, &code, self.line + 1, name).is_empty() {
            return false;
        }
        let imports = search::imports(Kind::TsJs, &text);
        let found = imports.iter().find_map(|search::Import { name, path }| {
            let (taken, module) = path.split_last()?;
            let whole = taken == "default" || taken == "*";
            let class = match object {
                Some(o) => (whole && name == o).then(|| word.to_owned()),
                None => (!whole && name == word).then(|| taken.clone()),
            }?;
            let file = search::css_module_file(&self.root, &self.files, here, module)?;
            Some((class, file))
        });
        let Some((class, file)) = found else {
            return false;
        };
        let reason = Reason::Import(file.display().to_string());
        self.styled_in(here, &class, false, vec![file], &reason);
        true
    }

    fn styled_in(
        &mut self,
        here: &Path,
        name: &str,
        id: bool,
        files: Vec<PathBuf>,
        reason: &Reason,
    ) {
        let piece = name
            .rsplit(['-', '_'])
            .find(|p| !p.is_empty())
            .unwrap_or(name);
        let mut found: Vec<Candidate> = Vec::new();
        for path in files {
            let Some(text) = self.file_text(&path).filter(|t| t.contains(piece)) else {
                continue;
            };
            let styles = match search::styles_in(&path) {
                Some(search::Styles::Stylesheet) => text.clone(),
                _ => search::style_blocks(&text),
            };
            let rules = if path.extension().is_some_and(|e| e == "sass") {
                search::sass_rules(&styles)
            } else {
                search::rules(&styles)
            };
            let lines: Vec<&str> = text.lines().collect();
            for rule in rules {
                let names = if id { &rule.ids } else { &rule.classes };
                let fresh = found
                    .last()
                    .is_none_or(|c| c.hit.path != path || c.hit.line1 != rule.line);
                if fresh && names.iter().any(|n| n == name) {
                    found.push(Candidate {
                        hit: Hit {
                            deleted: None,
                            path: path.clone(),
                            line1: rule.line,
                            byte_col: None,
                            text: lines
                                .get(rule.line - 1)
                                .copied()
                                .unwrap_or_default()
                                .to_owned(),
                        },
                        reason: reason.clone(),
                    });
                }
            }
        }
        self.truncated.set(false);
        self.show_definitions(Kind::Css, name, here, found, None);
    }

    fn scss_bound_in_block(&mut self, here: &Path, sheet: &Sheet) -> bool {
        let Sheet::Var(None, name) = sheet else {
            return false;
        };
        let text = self.buf.lines.join("\n");
        let Some(line1) = here
            .extension()
            .is_some_and(|e| e == "scss")
            .then(|| search::scss_block_binding(&text, self.line + 1, self.col, name))
            .flatten()
        else {
            return false;
        };
        let Some(text) = self.buf.lines.get(line1 - 1).cloned() else {
            return false;
        };
        let hit = Hit {
            deleted: None,
            path: here.to_path_buf(),
            line1,
            byte_col: None,
            text,
        };
        let found = vec![Candidate {
            hit,
            reason: Reason::Local,
        }];
        self.show_definitions(Kind::Css, name, here, found, None);
        true
    }

    fn sheet_declarations(&mut self, here: &Path, sheet: &Sheet) {
        let (word, ns) = match sheet {
            Sheet::Var(ns, n) | Sheet::Mixin(ns, n) | Sheet::Function(ns, n) => (n, ns.as_deref()),
            Sheet::Custom(n) | Sheet::LessVar(n) | Sheet::Placeholder(n) | Sheet::Keyframes(n) => {
                (n, None)
            }
            Sheet::Class(n) | Sheet::Id(n) | Sheet::Import(n) => (n, None),
        };
        if self.scss_bound_in_block(here, sheet) {
            return;
        }
        let Ok(re) = Regex::new(&search::sheet_patterns(sheet).join("|")) else {
            return;
        };
        let module = ns.and_then(|ns| {
            let uses = search::sass_uses(&self.buf.lines.join("\n"));
            let used = uses
                .into_iter()
                .find(|u| matches!(&u.namespace, search::SassNamespace::Named(n) if n == ns))?;
            self.sheet_module(here, &used.module)
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
        let wanted = |p: &Path| {
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or_default();
            match sheet {
                Sheet::LessVar(_) => ext == "less",
                Sheet::Var(..) | Sheet::Mixin(..) | Sheet::Function(..) | Sheet::Placeholder(_) => {
                    ext == "scss" || ext == "sass"
                }
                _ => search::styles_in(p).is_some(),
            }
        };
        if found.is_empty() {
            found = (self.files.iter())
                .filter(|p| wanted(p))
                .flat_map(|p| {
                    self.matching_lines(
                        p,
                        &re,
                        search::styles_in(p) == Some(search::Styles::StyleBlocksInMarkup),
                    )
                })
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect();
        }
        let dialect = here
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        let no_source = match sheet {
            Sheet::Function(None, n) => search::css_builtin(n),
            Sheet::Function(Some(_), _) | Sheet::Var(Some(_), _) | Sheet::Mixin(Some(_), _) => {
                module.is_none()
            }
            _ => false,
        };
        let outside = !no_source
            && match sheet {
                Sheet::Var(..) | Sheet::Mixin(..) | Sheet::Function(..) => {
                    dialect == "scss" || dialect == "sass"
                }
                Sheet::LessVar(_) => dialect == "less",
                _ => false,
            };
        if found.is_empty() && outside {
            let files: Vec<PathBuf> = (self.external_files(Kind::Css).iter())
                .filter(|p| wanted(p))
                .cloned()
                .collect();
            found = (self.grep_in(re.as_str(), &files).into_iter())
                .filter(|_| !files.is_empty())
                .filter(|hit| re.is_match(&search::css_code(&hit.text)))
                .map(|hit| Candidate {
                    hit,
                    reason: Reason::ByName,
                })
                .collect();
        }
        self.truncated.set(false);
        self.show_definitions(Kind::Css, word, here, found, None);
    }

    fn matching_lines(&self, path: &Path, re: &Regex, in_style_blocks_only: bool) -> Vec<Hit> {
        let Some(text) = self.file_text(path) else {
            return Vec::new();
        };
        let styles = if in_style_blocks_only {
            search::style_blocks(&text)
        } else {
            text.clone()
        };
        styles
            .lines()
            .zip(text.lines())
            .enumerate()
            .filter(|(_, (s, _))| re.is_match(&search::css_code(s)))
            .map(|(i, (_, t))| Hit {
                deleted: None,
                path: path.to_path_buf(),
                line1: i + 1,
                byte_col: None,
                text: t.to_owned(),
            })
            .collect()
    }

    fn sheet_module(&self, here: &Path, module: &str) -> Option<PathBuf> {
        let ext = here
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        let candidates = match ext {
            "scss" | "sass" => search::sass_candidates(module),
            _ => search::import_candidates(module, search::SheetSyntax::of_extension(ext)),
        };
        let dir = here.parent().unwrap_or_else(|| Path::new(""));
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
                    .map(|p| search::own_copy(&self.root, &p).unwrap_or(p))
            })
        })
    }

    fn follow_sheet_import(&mut self, here: &Path, module: &str) {
        if search::is_url(module) && !module.starts_with("sass:") {
            self.message = "no file in the project: a URL".into();
            return;
        }
        if module.starts_with("sass:") {
            self.message = format!("{module}: builtin, no source");
            return;
        }
        match self.sheet_module(here, module) {
            Some(path) => {
                let status = format!("path {}", path.display());
                self.open_link(&self.root.join(&path), 1, status);
            }
            None => self.message = format!("no file {module}"),
        }
    }

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
        let dir = here.parent().unwrap_or_else(|| Path::new(""));
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
            if files.is_empty() {
                self.message = format!("no file {path}");
            } else {
                let found = self.module_candidates(files);
                self.show_definitions(Kind::Html, path, here, found, None);
            }
            return;
        };
        let line = match fragment.filter(|f| !f.is_empty()) {
            None => 1,
            Some(f) => {
                if let Some(line) = self
                    .file_text(file)
                    .and_then(|t| search::element_with_id(&t, f))
                {
                    line
                } else {
                    self.message = format!("no id #{f} in {}", file.display());
                    return;
                }
            }
        };
        let shown = match path {
            "" => String::new(),
            _ => file.display().to_string(),
        };
        let anchor = fragment.map_or(String::new(), |f| format!("#{f}"));
        self.open_link(&self.root.join(file), line, format!("path {shown}{anchor}"));
    }
}
