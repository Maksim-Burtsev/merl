use super::definition::resolution;
use super::*;

use search::StarlarkLabel;

impl App {
    pub(super) fn starlark_definition(&mut self, here: &Path) -> bool {
        let text = self.buf.lines.join("\n");
        if search::literal_lines(Kind::Starlark, &text).get(self.line) == Some(&true) {
            return false;
        }
        let loads = search::starlark_loads(&text);
        let line = self.line_str().to_owned();
        if let Some((range, plain)) = search::starlark_string_at(&line, self.col) {
            let label = &line[range.clone()];
            let key = line
                .get(range.end + 1..)
                .is_some_and(|rest| rest.trim_start().starts_with(':'));
            let named = loads
                .iter()
                .any(|l| l.name_strings.contains(&(self.line, range.start)));
            let found = (plain && !key && !named)
                .then(|| self.label_places(here, label))
                .flatten();
            match found {
                Some((word, found)) if !found.is_empty() => {
                    self.show_definitions(Kind::Starlark, &word, here, found, None)
                }
                _ => self.message = resolution(label, None, &[], None, false),
            }
            return true;
        }
        let Some((range, word)) = self.definition_word(Some(Kind::Starlark)) else {
            return false;
        };
        if line[..range.start].ends_with('.') {
            return false;
        }
        let in_load = loads.iter().any(|l| l.lines.contains(&self.line));
        if search::keyword_argument(&text, self.line + 1, &range)
            && !in_load
            && !line.trim_start().starts_with("def ")
        {
            self.message = resolution(&word, None, &[], None, false);
            return true;
        }
        if !search::bindings(Kind::Starlark, &text, self.line + 1, &word).is_empty() {
            return false;
        }
        let Some((label, taken)) = loads.iter().find_map(|l| {
            let (_, taken) = l.names.iter().find(|(local, _)| *local == word)?;
            Some((l.label.clone(), taken.clone()))
        }) else {
            return false;
        };
        let Some(file) = self.label_file(here, &label) else {
            return false;
        };
        let Some(loaded) = self.file_text(&file) else {
            return false;
        };
        let via = match file.is_absolute() {
            true => label,
            false => file.display().to_string(),
        };
        let found: Vec<Candidate> = search::starlark_declarations(&loaded, &taken)
            .into_iter()
            .map(|n| Candidate {
                hit: Hit {
                    deleted: None,
                    path: file.clone(),
                    line1: n,
                    byte_col: None,
                    text: loaded.lines().nth(n - 1).unwrap_or_default().to_owned(),
                },
                reason: Reason::Import(via.clone()),
            })
            .collect();
        if found.is_empty() {
            return false;
        }
        self.show_definitions(Kind::Starlark, &taken, here, found, None);
        true
    }

    fn label_file(&self, here: &Path, label: &str) -> Option<PathBuf> {
        let (_, found) = self.label_places(here, label)?;
        match found.as_slice() {
            [c] if matches!(c.reason, Reason::Module(_)) => Some(c.hit.path.clone()),
            _ => None,
        }
    }

    fn label_places(&self, here: &Path, label: &str) -> Option<(String, Vec<Candidate>)> {
        const ROOTS: [&str; 3] = ["MODULE.bazel", "WORKSPACE.bazel", "WORKSPACE"];
        let label = search::starlark_label(label)?;
        let here = self.root.join(here);
        let own = (!here.starts_with(&self.root))
            .then(|| {
                here.ancestors()
                    .find(|a| a.parent().and_then(Path::file_name) == Some("external".as_ref()))
            })
            .flatten();
        let main = here
            .ancestors()
            .skip(1)
            .take_while(|d| d.starts_with(&self.root))
            .find(|d| ROOTS.iter().any(|f| d.join(f).is_file()))
            .unwrap_or(&self.root)
            .to_path_buf();
        let external = match own {
            Some(repo) => repo.parent().map(Path::to_path_buf),
            None => search::starlark_external(&main),
        };
        let own = own.map(Path::to_path_buf);
        let shown = |p: PathBuf| {
            p.strip_prefix(&self.root)
                .map(Path::to_path_buf)
                .unwrap_or(p)
        };
        let read = |p: &Path| self.file_text(&shown(p.to_path_buf()));
        let line_of = |path: PathBuf, line: usize, reason: Reason| {
            let text = read(&path)?;
            let hit = Hit {
                deleted: None,
                text: text.lines().nth(line - 1)?.to_owned(),
                path: shown(path),
                line1: line,
                byte_col: None,
            };
            Some(Candidate { hit, reason })
        };
        let at_root = |name: &str, find: &dyn Fn(&str) -> Option<usize>| {
            ROOTS.into_iter().find_map(|f| {
                let path = main.join(f);
                let line = find(&read(&path)?)?;
                line_of(path, line, Reason::Label(name.to_owned()))
            })
        };
        let (repo, package, name) = match label {
            StarlarkLabel::Repo(repo) => {
                let found = at_root("repository", &|t| search::starlark_repo_line(t, &repo));
                return Some((repo, found.into_iter().collect()));
            }
            StarlarkLabel::Target {
                repo,
                package,
                name,
            } => (repo, package, name),
        };
        let project = |r: &String| {
            ROOTS
                .into_iter()
                .flat_map(|f| read(&main.join(f)))
                .any(|t| search::starlark_project_names(&t).contains(r))
        };
        let base = match repo {
            Some(r) if r.is_empty() || project(&r) => main,
            Some(r) => search::starlark_repo_dir(external.as_ref()?, &r)?,
            None => own.unwrap_or(main),
        };
        let dir = match package {
            Some(p) => base.join(p),
            None => here.parent()?.to_path_buf(),
        };
        let target = ["BUILD.bazel", "BUILD", "BUCK"].into_iter().find_map(|f| {
            let path = dir.join(f);
            let line = search::starlark_target_line(&read(&path)?, &name)?;
            line_of(path, line, Reason::Label("target".to_owned()))
        });
        let file = dir.join(&name);
        let found = match target {
            Some(t) => vec![t],
            None if file.is_file() => {
                let path = shown(file);
                self.module_candidates(vec![path])
            }
            None => Vec::new(),
        };
        let word = name.rsplit('/').next().unwrap_or(&name).to_owned();
        Some((word, found))
    }
}
