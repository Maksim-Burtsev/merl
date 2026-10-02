//! `d` in Markdown (#421): a link, a reference or an `#anchor` opens the file or the heading it
//! names, and a code span naming a file of the project opens that file.

use super::*;

use search::MdAt;

impl App {
    pub(super) fn follow_markdown(&mut self, here: &Path) {
        // On a deleted line of a review, that line is read where it is drawn.
        let mut lines: Vec<&str> = self.buf.lines.iter().map(String::as_str).collect();
        if self.deleted.is_some() {
            lines.insert(self.line, self.line_str());
        }
        let at = lines[..self.line]
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + self.col;
        match search::markdown_at(&lines.join("\n"), at) {
            MdAt::Link(target) => self.follow_link(here, &target),
            MdAt::Code(span) => self.follow_code_path(here, &span),
            MdAt::Nothing => self.message = "no link here".into(),
        }
    }

    /// A link's `target`: a path from `here`'s directory (or the root, after a `/`), an
    /// `#anchor` of that file or of `here`, GitHub's `#L12` line anchors.
    fn follow_link(&mut self, here: &Path, target: &str) {
        static URL: std::sync::LazyLock<Regex> =
            std::sync::LazyLock::new(|| Regex::new(r"^(?:[A-Za-z][A-Za-z0-9+.-]*:|//)").unwrap());
        static LINES: std::sync::LazyLock<Regex> =
            std::sync::LazyLock::new(|| Regex::new(r"^L(\d+)(?:-L\d+)?$").unwrap());
        if URL.is_match(target) {
            self.message = "link outside the project".into();
            return;
        }
        let (path, anchor) = match target.split_once('#') {
            Some((p, a)) => (search::percent_decoded(p), Some(search::percent_decoded(a))),
            None => (search::percent_decoded(target), None),
        };
        let dir = here.parent().unwrap_or(Path::new(""));
        let Some(rel) = (match path.as_str() {
            "" => Some(here.to_path_buf()),
            p => in_project(dir, p),
        }) else {
            self.message = "link outside the project".into();
            return;
        };
        let full = self.root.join(&rel);
        if full.is_dir() {
            self.say_about(&full, "/: a directory");
            return;
        }
        if !full.is_file() {
            self.message = format!("no file {}", rel.display());
            return;
        }
        let shown = match path.as_str() {
            "" => String::new(),
            _ => rel.display().to_string(),
        };
        let line = match &anchor {
            None => 1,
            Some(a) => match LINES.captures(a) {
                Some(c) => c[1].parse().unwrap_or(1),
                None => match self.text_of(&rel).and_then(|t| search::anchor_line(&t, a)) {
                    Some(line) => line,
                    None => {
                        self.message = format!("no heading #{a} in {}", rel.display());
                        return;
                    }
                },
            },
        };
        let anchor = anchor.map_or(String::new(), |a| format!("#{a}"));
        self.open_link(&full, line, format!("link {shown}{anchor}"));
    }

    /// A code span that names a file of the project, from the root or from `here`'s directory,
    /// with `:line` or `:line:col` after it or not; a bare file name that several files carry is
    /// a picker of them.
    fn follow_code_path(&mut self, here: &Path, span: &str) {
        static AT: std::sync::LazyLock<Regex> =
            std::sync::LazyLock::new(|| Regex::new(r"^(.+?)(?::(\d+))?(?::\d+)?$").unwrap());
        let Some(c) = AT.captures(span.trim()) else {
            self.message = "no link here".into();
            return;
        };
        let (name, line) = (&c[1], c.get(2).and_then(|n| n.as_str().parse().ok()));
        let dir = here.parent().unwrap_or(Path::new(""));
        let file = [Path::new(""), dir]
            .into_iter()
            .filter_map(|d| in_project(d, name))
            .find(|rel| self.files.contains(rel));
        let files: Vec<PathBuf> = match file {
            Some(rel) => vec![rel],
            None if !name.contains('/') => self
                .files
                .iter()
                .filter(|f| f.file_name().is_some_and(|n| n == name))
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        match files.as_slice() {
            [] => self.message = "no link here".into(),
            [rel] => {
                let at = line.map_or(String::new(), |n| format!(":{n}"));
                let status = format!("path {}{at}", rel.display());
                self.open_link(&self.root.join(rel), line.unwrap_or(1), status);
            }
            _ => {
                let status = format!("path {name}: {} files", files.len());
                let found = files
                    .into_iter()
                    .map(|path| Candidate {
                        hit: Hit {
                            path,
                            line: line.unwrap_or(1),
                            col: 0,
                            deleted: None,
                            text: String::new(),
                        },
                        reason: Reason::ByName,
                    })
                    .collect();
                let items = self.definition_items(Kind::Markdown, name, found);
                self.show_picker(PickerKind::Definitions, items);
                self.message = status;
            }
        }
    }

    pub(super) fn open_link(&mut self, path: &Path, line: usize, status: String) {
        self.jump_to(path, line);
        if self.buf.path.as_deref() == Some(path) {
            self.message = status;
        }
    }
}

/// `p` from `dir`, both inside the project, as a path from its root; a leading `/` is the root.
/// `None` when it climbs out of the project.
pub(super) fn in_project(dir: &Path, p: &str) -> Option<PathBuf> {
    let joined = match p.strip_prefix('/') {
        Some(from_root) => PathBuf::from(from_root),
        None => dir.join(p),
    };
    let mut out = PathBuf::new();
    for c in joined.components() {
        match c {
            Component::Normal(name) => out.push(name),
            Component::ParentDir if out.pop() => {}
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(out)
}
