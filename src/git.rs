use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
#[cfg(not(target_family = "wasm"))]
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::real::canonicalize;
use crate::tree::Order;
#[cfg(target_family = "wasm")]
use crate::web::GitCommand as Command;
use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Added,
    Changed,
    DeletedBelow,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextLine {
    File(usize),
    Deleted(usize, usize),
}

impl TextLine {
    pub fn key(self) -> usize {
        match self {
            Self::File(l) | Self::Deleted(l, _) => l,
        }
    }
}

impl Ord for TextLine {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let order = |t: &Self| match *t {
            Self::File(l) => (l, usize::MAX),
            Self::Deleted(k, i) => (k, i),
        };
        order(self).cmp(&order(other))
    }
}

impl PartialOrd for TextLine {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedLine {
    pub path: PathBuf,
    pub line: usize,
    pub at: TextLine,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedRun {
    pub path: PathBuf,
    pub line: usize,
    pub len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    pub marks: HashMap<usize, Mark>,
    pub ghosts: BTreeMap<usize, Vec<String>>,
    pub hunks: Vec<TextLine>,
    pub ghost_from: BTreeMap<usize, usize>,
    pub pairs: HashMap<usize, (usize, usize)>,
}

pub fn diff(root: &Path, path: &Path, base: Option<&str>, old: Option<&Path>) -> Diff {
    let mut cmd = Command::new("git");
    cmd.arg("--literal-pathspecs");
    cmd.arg("-C")
        .arg(root)
        .args(["diff", "-U0", "-M", "--no-color", "--no-ext-diff"])
        .arg("--inter-hunk-context=0");
    if let Some(base) = base {
        cmd.arg(base);
    }
    cmd.arg("--").arg(path);
    if let Some(old) = old {
        cmd.arg(old);
    }
    let out = cmd.output();
    match out {
        Ok(o) if o.status.success() => parse(&String::from_utf8_lossy(&o.stdout), base.is_some()),
        _ => Diff::default(),
    }
}

fn parse(diff: &str, review: bool) -> Diff {
    let mut out = Diff::default();
    let mut lines = diff.lines().peekable();
    let mut key_after_last_hunk = None;
    while let Some(line) = lines.next() {
        let Some(hunk) = line.strip_prefix("@@ ") else {
            continue;
        };
        let Some((old, new)) = hunk.split_once(" +") else {
            continue;
        };
        let (Some((old_start1, old_n)), Some((new_start1, new_n))) = (
            start_and_count(old.trim_start_matches('-')),
            start_and_count(new.split(' ').next().unwrap_or("")),
        ) else {
            continue;
        };
        let first_new_line = if new_n == 0 {
            new_start1
        } else {
            new_start1 - 1
        };
        if review {
            let mut deleted = Vec::new();
            let mut added = Vec::new();
            while let Some(l) =
                lines.next_if(|l| l.starts_with('-') || l.starts_with('+') || l.starts_with('\\'))
            {
                if let Some(text) = l.strip_prefix('-') {
                    deleted.push(text.to_owned());
                } else if let Some(text) = l.strip_prefix('+') {
                    added.push(text.to_owned());
                }
            }
            let mut first = TextLine::File(first_new_line);
            if !deleted.is_empty() {
                let ghosts = out.ghosts.entry(first_new_line).or_default();
                let offset = ghosts.len();
                if added.is_empty() {
                    first = TextLine::Deleted(first_new_line, offset);
                }
                out.ghost_from
                    .entry(first_new_line)
                    .or_insert(old_start1 - 1);
                for (i, j) in crate::intraline::pair(&deleted, &added) {
                    out.pairs
                        .insert(first_new_line + j, (first_new_line, offset + i));
                }
                ghosts.extend(deleted);
            }
            if key_after_last_hunk != Some(first_new_line) {
                out.hunks.push(first);
            }
            key_after_last_hunk = Some(first_new_line + new_n);
            for l in first_new_line..first_new_line + new_n {
                out.marks.insert(l, Mark::Added);
            }
            continue;
        }
        if new_n == 0 {
            out.marks
                .insert(new_start1.saturating_sub(1), Mark::DeletedBelow);
            continue;
        }
        let mark = if old_n == 0 {
            Mark::Added
        } else {
            Mark::Changed
        };
        for l in new_start1 - 1..new_start1 - 1 + new_n {
            out.marks.insert(l, mark);
        }
    }
    out
}

fn start_and_count(s: &str) -> Option<(usize, usize)> {
    match s.split_once(',') {
        Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
        None => Some((s.parse().ok()?, 1)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent fact about the file"
)]
pub struct ReviewFile {
    pub path: PathBuf,
    pub status: char,
    pub old: Option<PathBuf>,
    pub added: usize,
    pub deleted: usize,
    pub binary: bool,
    pub untracked: bool,
    pub generated: bool,
    pub same_bytes: bool,
}

impl ReviewFile {
    pub fn has_hunks(&self) -> bool {
        self.added + self.deleted > 0
    }

    pub fn is_picture(&self) -> bool {
        self.binary && !self.same_bytes && crate::picture::raster_name(&self.path).is_some()
    }

    pub fn is_stop(&self, root: &Path, pictures: bool) -> bool {
        self.has_hunks()
            || pictures && self.is_picture()
            || !(self.binary || root.join(&self.path).is_dir())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    pub branch: String,
    pub base: String,
    pub merge_base: String,
    pub files: Vec<ReviewFile>,
    pub deleted: Arc<Vec<DeletedLine>>,
    pub added: Arc<Vec<AddedRun>>,
    pub note: Option<String>,
}

impl Review {
    pub fn open(root: &Path, branch: Option<&str>, base: Option<&str>) -> Result<Self> {
        let git = |args: &[&str]| git(root, args);
        let base = match base {
            Some(b) => b.to_owned(),
            None => detect_base(&git)?,
        };
        let note = match branch {
            Some(b) => checkout(&git, b, &base)?,
            None => None,
        };
        let r = Self::list(root, base)?;
        if r.files.is_empty() {
            bail!("{} has no changes against {}", r.branch, r.base);
        }
        Ok(Self { note, ..r })
    }

    pub fn open_branch(root: &Path, b: &str, base: Option<&str>) -> Result<(PathBuf, Self)> {
        let git = |args: &[&str]| git(root, args);
        let on = |h: String| h == b || b.strip_prefix("origin/") == Some(h.as_str());
        if git(&["symbolic-ref", "-q", "--short", "HEAD"]).is_ok_and(on) {
            return Ok((root.to_path_buf(), Self::open(root, Some(b), base)?));
        }
        let base = match base {
            Some(b) => b.to_owned(),
            None => detect_base(&git)?,
        };
        if let Some((remote, name)) = other_remote(&git, b)
            && git(&["fetch", "-q", "origin", b]).is_err()
        {
            let fetched = git(&["fetch", "-q", &remote, name]).is_ok();
            if let Some(main) = base.strip_prefix("origin/") {
                drop(git(&["fetch", "-q", "origin", main]));
            }
            let at = format!("refs/remotes/{b}");
            if git(&["rev-parse", "--verify", "-q", &at]).is_err() {
                bail!("no branch {name} on {remote}");
            }
            let folder = check_out_folder(root, b, &at)?;
            let r = Self::open(&folder, None, Some(&base))?;
            let note = (!fetched).then(|| format!("{b} not fetched"));
            return Ok((folder, Self { note, ..r }));
        }
        let (b, fetched) = fetch(&git, b, &base)?;
        let (remote, local) = (format!("origin/{b}"), format!("refs/heads/{b}"));
        let has_local = git(&["rev-parse", "--verify", "-q", &local]).is_ok();
        let tracked = format!("refs/remotes/{remote}");
        if !fetched && !has_local && git(&["rev-parse", "--verify", "-q", &tracked]).is_err() {
            bail!("no branch {b} on origin");
        }
        let (at, note) = if !fetched {
            let at = if has_local { local } else { tracked };
            (at, Some(format!("{remote} not fetched")))
        } else if has_local && own_commits(&git, b) {
            let ahead = git(&["merge-base", "--is-ancestor", &remote, &local]).is_ok();
            let note = (!ahead).then(|| lag(&git, &remote, &local));
            (local, note)
        } else {
            (tracked, None)
        };
        let folder = check_out_folder(root, b, &at)?;
        let r = Self::open(&folder, None, Some(&base))?;
        Ok((folder, Self { note, ..r }))
    }

    pub fn first_file(&self, root: &Path, pictures: bool) -> Option<PathBuf> {
        if let Some(f) = self.files.iter().find(|f| f.is_stop(root, pictures)) {
            return (f.status != 'D').then(|| root.join(&f.path));
        }
        (self.files.iter().filter(|f| f.status != 'D'))
            .map(|f| root.join(&f.path))
            .find(|p| p.is_file())
    }

    pub fn refresh(&self, root: &Path) -> Result<Self> {
        Self::list(root, self.base.clone())
    }

    fn list(root: &Path, base: String) -> Result<Self> {
        let git = |args: &[&str]| git(root, args);
        let head = git(&["symbolic-ref", "-q", "HEAD"]).or_else(|_| rebasing(&git, root));
        let folder = dirs(root).as_ref().and_then(folder_branch);
        let branch = (head.as_deref().ok())
            .and_then(|h| h.strip_prefix("refs/heads/"))
            .map(str::to_owned)
            .or_else(|| folder.clone())
            .unwrap_or_else(|| "HEAD".to_owned());
        let merge_base = git(&["merge-base", &base, "HEAD"])
            .with_context(|| format!("no merge base between {base} and HEAD"))?;
        let raw = ["diff", "--raw", "--no-abbrev", "-z", &merge_base];
        let mut files = parse_name_status(&git(&raw)?);
        let numstat = [
            "diff",
            "--numstat",
            "--ignore-submodules",
            "-z",
            &merge_base,
        ];
        let dir_link = |p: &Path| p.is_symlink() && p.is_dir();
        for (path, counts) in parse_numstat(&git(&numstat)?) {
            if let Some(f) = files.iter_mut().find(|f| f.path == path)
                && !dir_link(&root.join(&path))
            {
                f.binary = counts.is_none();
                (f.added, f.deleted) = counts.unwrap_or((0, 0));
            }
        }
        let others = git(&["ls-files", "--others", "--exclude-standard", "-z"])?;
        for path in others
            .split('\0')
            .filter(|p| !p.is_empty() && !p.ends_with('/'))
            .filter(|p| folder.is_none() || !root.join(p).is_symlink())
        {
            files.retain(|f| f.path != Path::new(path));
            files.push(untracked(root, Path::new(path)));
        }
        let attrs = generated_attrs(root, &files).unwrap_or_default();
        for f in &mut files {
            f.generated = match attrs.get(&f.path) {
                Some(&said) => said,
                None => generated(root, f),
            };
        }
        let order = match (branch.as_str(), dirs(root)) {
            _ if !crate::tree::orders_on() => Order::default(),
            ("HEAD", _) | (_, None) => Order::default(),
            (b, Some(dirs)) => Order::read(
                &dirs.common_refs_dir.join("merl").join("review").join(b),
                Path::new(""),
            ),
        };
        files.sort_by_cached_key(|f| crate::tree::sort_key(&f.path, false, &order));
        let patch = git(&[
            "-c",
            "core.quotePath=false",
            "diff",
            "-U0",
            "--no-color",
            "--no-ext-diff",
            "--ignore-submodules",
            "--inter-hunk-context=0",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            &merge_base,
        ])?;
        let (deleted, added) = changed_lines(root, &patch, &files);
        let (deleted, added) = (Arc::new(deleted), Arc::new(added));
        Ok(Self {
            branch,
            base,
            merge_base,
            files,
            deleted,
            added,
            note: None,
        })
    }

    pub fn diff(&self, root: &Path, path: &Path, file: Option<&ReviewFile>) -> Diff {
        match file {
            Some(f) if f.untracked => {
                let n = count_lines(path).map_or(0, |(_, n)| n);
                Diff {
                    marks: (0..n).map(|l| (l, Mark::Added)).collect(),
                    hunks: Vec::from_iter((n > 0).then_some(TextLine::File(0))),
                    ..Default::default()
                }
            }
            _ => diff(
                root,
                path,
                Some(&self.merge_base),
                file.and_then(|f| f.old.as_deref()),
            ),
        }
    }

    pub fn branch_name(&self) -> Option<&str> {
        (self.branch != "HEAD").then_some(self.branch.as_str())
    }

    pub fn branch_or_commit(&self, root: &Path) -> String {
        match self.branch.as_str() {
            "HEAD" => {
                git(root, &["rev-parse", "--short", "HEAD"]).unwrap_or_else(|_| self.branch.clone())
            }
            branch => branch.to_owned(),
        }
    }

    pub fn file(&self, rel: &Path) -> Option<&ReviewFile> {
        self.files.iter().find(|f| f.path == rel)
    }

    pub fn base_bytes(&self, root: &Path, rel: &Path) -> Result<Vec<u8>> {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .arg("show")
            .arg(format!("{}:{}", self.merge_base, rel.display()))
            .output()?;
        if !out.status.success() {
            bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
        }
        Ok(out.stdout)
    }

    pub fn base_file(&self, root: &Path, rel: &Path) -> Result<PathBuf> {
        let parent = dirs(root)
            .context("no git directory")?
            .worktree_head_dir
            .join("merl/old");
        let path = parent.join(&self.merge_base).join(rel);
        if path.is_file() {
            return Ok(path);
        }
        let bytes = self.base_bytes(root, rel)?;
        if let Ok(old) = std::fs::read_dir(&parent) {
            for e in old.flatten().filter(|e| e.file_name() != *self.merge_base) {
                drop(std::fs::remove_dir_all(e.path()));
            }
        }
        std::fs::create_dir_all(path.parent().context("no parent")?)?;
        let tmp = path.with_file_name(format!(".{}", crate::real::pid()));
        let written = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, &path));
        if written.is_err() {
            drop(std::fs::remove_file(&tmp));
        }
        written?;
        Ok(path)
    }

    pub fn base_tree(&self, root: &Path) -> Result<PathBuf> {
        let git_dir = dirs(root).context("no git directory")?.worktree_head_dir;
        let changed: std::collections::HashSet<&Path> = (self.files.iter())
            .map(|f| f.old.as_deref().unwrap_or(&f.path))
            .collect();
        let mut stamp: Vec<String> = changed.iter().map(|p| p.display().to_string()).collect();
        stamp.sort();
        stamp.insert(0, self.merge_base.clone());
        let stamp = stamp.join("\n");
        let parent = git_dir.join("merl").join("base");
        let dir = parent.join(&self.merge_base);
        if std::fs::read_to_string(dir.join(".merl-stamp")).is_ok_and(|s| s == stamp) {
            return Ok(dir);
        }
        let tmp = parent.join(format!(".{}-{}", self.merge_base, crate::real::pid()));
        drop(std::fs::remove_dir_all(&tmp));
        std::fs::create_dir_all(&tmp)?;
        let listing = git(
            root,
            &["ls-tree", "-r", "-z", "--full-tree", &self.merge_base],
        )?;
        let mut from_git = Vec::new();
        for entry in listing.split('\0').filter(|e| !e.is_empty()) {
            let Some((meta, path)) = entry.split_once('\t') else {
                continue;
            };
            let (mode, path) = (meta.split(' ').next().unwrap_or(""), Path::new(path));
            let dst = tmp.join(path);
            if let Some(d) = dst.parent() {
                std::fs::create_dir_all(d)?;
            }
            match mode {
                "160000" => {}
                "120000" => from_git.push((path.to_path_buf(), true)),
                _ if changed.contains(path) => from_git.push((path.to_path_buf(), false)),
                _ => {
                    if std::fs::copy(root.join(path), &dst).is_err() {
                        from_git.push((path.to_path_buf(), false));
                    }
                }
            }
        }
        for (path, link) in from_git {
            let spec = format!("{}:{}", self.merge_base, path.display());
            let out = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["cat-file", "blob", &spec])
                .output()?;
            #[cfg(unix)]
            if link {
                let target = String::from_utf8_lossy(&out.stdout).into_owned();
                drop(std::os::unix::fs::symlink(target, tmp.join(&path)));
                continue;
            }
            std::fs::write(tmp.join(&path), out.stdout)?;
        }
        #[cfg(unix)]
        {
            let ignored = git(
                root,
                &[
                    "ls-files",
                    "-z",
                    "--others",
                    "--ignored",
                    "--exclude-standard",
                    "--directory",
                ],
            )
            .unwrap_or_default();
            let dir =
                |d: &str| d.ends_with('/') || root.join(d).is_symlink() && root.join(d).is_dir();
            for d in ignored.split('\0').filter(|d| dir(d)) {
                let d = Path::new(d.trim_end_matches('/'));
                let dst = tmp.join(d);
                if dst.parent().is_some_and(Path::exists) && !dst.exists() {
                    drop(std::os::unix::fs::symlink(root.join(d), dst));
                }
            }
        }
        std::fs::write(tmp.join(".merl-stamp"), &stamp)?;
        if let Ok(old) = std::fs::read_dir(&parent) {
            for e in old.flatten().filter(|e| e.path() != tmp) {
                drop(std::fs::remove_dir_all(e.path()));
            }
        }
        if std::fs::rename(&tmp, &dir).is_err() {
            drop(std::fs::remove_dir_all(&tmp));
        }
        Ok(dir)
    }
}

fn changed_lines(
    root: &Path,
    patch: &str,
    files: &[ReviewFile],
) -> (Vec<DeletedLine>, Vec<AddedRun>) {
    let mut parts: HashMap<PathBuf, String> = HashMap::new();
    let (mut name, mut part, mut header) = (None::<PathBuf>, String::new(), true);
    let mut done = |name: Option<PathBuf>, part: &mut String| {
        if let Some(name) = name {
            parts.insert(name, std::mem::take(part));
        }
        part.clear();
    };
    for line in patch.lines() {
        if line.starts_with("diff --git ") {
            done(name.take(), &mut part);
            header = true;
            continue;
        }
        if header {
            header = !line.starts_with("@@ ");
            let named = |prefix: &str| {
                let n = unquote(line.strip_prefix(prefix)?.trim_end_matches('\t'));
                let n = n.strip_prefix("a/").or_else(|| n.strip_prefix("b/"))?;
                Some(PathBuf::from(n))
            };
            if let Some(n) = named("+++ ").or_else(|| named("--- ").filter(|_| name.is_none())) {
                name = Some(n);
            }
        }
        part.push_str(line);
        part.push('\n');
    }
    done(name, &mut part);
    let (mut out, mut added) = (Vec::new(), Vec::new());
    for f in files {
        if f.untracked {
            let len = count_lines(&root.join(&f.path)).map_or(0, |(_, n)| n);
            added.push(AddedRun {
                path: f.path.clone(),
                line: 1,
                len,
            });
            continue;
        }
        let Some(diff) = parts.get(&f.path).map(|p| parse(p, true)) else {
            continue;
        };
        let mut lines: Vec<usize> = (diff.marks.iter())
            .filter(|(_, m)| matches!(m, Mark::Added | Mark::Changed))
            .map(|(&l, _)| l)
            .collect();
        lines.sort_unstable();
        for l in lines {
            match added.last_mut() {
                Some(AddedRun { path, line, len }) if *path == f.path && *line + *len == l + 1 => {
                    *len += 1;
                }
                _ => added.push(AddedRun {
                    path: f.path.clone(),
                    line: l + 1,
                    len: 1,
                }),
            }
        }
        for (&k, ghosts) in &diff.ghosts {
            let from = diff.ghost_from.get(&k).copied().unwrap_or(0);
            out.extend(ghosts.iter().enumerate().map(|(i, text)| DeletedLine {
                path: f.path.clone(),
                line: from + i + 1,
                at: match f.status {
                    'D' => TextLine::File(from + i),
                    _ => TextLine::Deleted(k, i),
                },
                text: text.clone(),
            }));
        }
    }
    (out, added)
}

fn unquote(s: &str) -> String {
    let Some(inner) = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return s.to_owned();
    };
    let b = inner.as_bytes();
    let (mut out, mut i) = (Vec::new(), 0);
    while i < b.len() {
        if b[i] != b'\\' || i + 1 == b.len() {
            out.push(b[i]);
            i += 1;
            continue;
        }
        i += 2;
        out.push(match b[i - 1] {
            b'n' => b'\n',
            b't' => b'\t',
            b'r' => b'\r',
            b'a' => 7,
            b'b' => 8,
            b'v' => 11,
            b'f' => 12,
            b'0'..=b'7' => {
                let end = (i + 2).min(b.len());
                let digits = std::str::from_utf8(&b[i - 1..end]).unwrap_or("");
                let n = u8::from_str_radix(digits, 8).unwrap_or(b'?');
                i = end;
                n
            }
            c => c,
        });
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub(crate) fn git(root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .context("cannot run git: --review needs it on PATH")?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .trim_end_matches('\n')
        .to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDirs {
    pub worktree_head_dir: PathBuf,
    pub common_refs_dir: PathBuf,
}

pub fn dirs(root: &Path) -> Option<GitDirs> {
    let out = git(root, &["rev-parse", "--git-dir", "--git-common-dir"]).ok()?;
    let mut dirs = out.lines().map(|d| canonicalize(&root.join(d)).ok());
    Some(GitDirs {
        worktree_head_dir: dirs.next()??,
        common_refs_dir: dirs.next()??,
    })
}

pub fn changed_files(root: &Path) -> HashMap<PathBuf, char> {
    let status = [
        "--no-optional-locks",
        "status",
        "--porcelain",
        "-z",
        "--untracked-files=all",
        "--no-renames",
        "--",
        ".",
    ];
    let (Ok(prefix), Ok(out)) = (
        git(root, &["rev-parse", "--show-prefix"]),
        git(root, &status),
    ) else {
        return HashMap::new();
    };
    parse_status(&out, Path::new(&prefix))
}

fn parse_status(out: &str, prefix: &Path) -> HashMap<PathBuf, char> {
    let mut marks = HashMap::new();
    for entry in out.split('\0') {
        let (Some(xy), Some(path)) = (entry.get(..2), entry.get(3..)) else {
            continue;
        };
        let Ok(rel) = Path::new(path).strip_prefix(prefix) else {
            continue;
        };
        let mark = match xy {
            "??" | "UA" | "DU" | " A" => 'A',
            _ if xy.starts_with('A') && !matches!(xy, "AA" | "AU") => 'A',
            _ => 'M',
        };
        marks.entry(rel.to_path_buf()).or_insert(mark);
    }
    marks
}

pub fn worktree_of(root: &Path, branch: &str) -> Option<PathBuf> {
    let out = git(root, &["worktree", "list", "--porcelain"]).ok()?;
    let here = root.canonicalize().ok()?;
    let head = format!("branch refs/heads/{branch}");
    out.split("\n\n")
        .filter(|w| w.lines().any(|l| l == head) && !w.lines().any(|l| l.starts_with("prunable")))
        .filter_map(|w| {
            w.lines()
                .next()?
                .strip_prefix("worktree ")
                .map(PathBuf::from)
        })
        .find(|p| p.canonicalize().is_ok_and(|p| p != here))
}

fn rebasing(git: &dyn Fn(&[&str]) -> Result<String>, root: &Path) -> Result<String> {
    let merge = "rebase-merge/head-name";
    let paths = git(&[
        "rev-parse",
        "--git-path",
        merge,
        "--git-path",
        "rebase-apply/head-name",
    ])?;
    (paths.lines())
        .find_map(|p| std::fs::read_to_string(root.join(p)).ok())
        .map(|name| name.trim().to_owned())
        .context("no rebase in progress")
}

fn untracked(root: &Path, path: &Path) -> ReviewFile {
    let (binary, added) = count_lines(&root.join(path)).unwrap_or((false, 0));
    ReviewFile {
        path: path.to_path_buf(),
        status: 'A',
        old: None,
        added,
        deleted: 0,
        binary,
        untracked: true,
        generated: false,
        same_bytes: false,
    }
}

const GENERATED_NAMES: &[&str] = &[
    "package-lock.json",
    "npm-shrinkwrap.json",
    "pnpm-lock.yaml",
    "poetry.lock",
    "uv.lock",
    "pdm.lock",
    "Pipfile.lock",
    "Cargo.lock",
    "Cargo.toml.orig",
    "composer.lock",
    "flake.lock",
    "deno.lock",
    "Gopkg.lock",
    "glide.lock",
    "MODULE.bazel.lock",
    ".terraform.lock.hcl",
    ".pnp.js",
    ".pnp.cjs",
    ".pnp.mjs",
    ".pnp.loader.mjs",
];

const GENERATED_ENDS: &[&str] = &[".min.js", ".min.css", ".js.map", ".css.map"];

const GENERATED_DIRS: &[&str] = &["node_modules", "Godeps"];

#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "extensions match as written, as kind_of does; ignoring case would change which files merl recognises (#873)"
)]
fn generated(root: &Path, f: &ReviewFile) -> bool {
    use std::io::{BufRead, BufReader};
    let name = f.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let in_dir = f.path.parent().is_some_and(|dir| {
        dir.components()
            .any(|c| GENERATED_DIRS.contains(&&*c.as_os_str().to_string_lossy()))
    });
    if GENERATED_NAMES.contains(&name) || GENERATED_ENDS.iter().any(|e| name.ends_with(e)) || in_dir
    {
        return true;
    }
    let path = root.join(&f.path);
    if !name.ends_with(".go")
        || f.status == 'D'
        || f.binary
        || !std::fs::metadata(&path).is_ok_and(|m| m.is_file())
    {
        return false;
    }
    let Ok(file) = std::fs::File::open(&path) else {
        return false;
    };
    (BufReader::new(file).lines().take(40).map_while(Result::ok)).any(|l| {
        let l = l.trim_end();
        l.starts_with("// Code generated ") && l.ends_with(" DO NOT EDIT.")
    })
}

fn generated_attrs(root: &Path, files: &[ReviewFile]) -> Result<HashMap<PathBuf, bool>> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-attr", "-z", "--stdin"])
        .args(["linguist-generated", "gitlab-generated"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut paths = Vec::new();
    for f in files {
        paths.extend_from_slice(f.path.to_string_lossy().as_bytes());
        paths.push(0);
    }
    let mut stdin = child.stdin.take().context("no stdin")?;
    let writer = std::thread::spawn(move || stdin.write_all(&paths));
    let out = child.wait_with_output()?;
    writer.join().ok();
    let out = String::from_utf8_lossy(&out.stdout);
    let fields: Vec<&str> = out.split('\0').collect();
    let mut said = HashMap::new();
    for [path, _, value] in fields.as_chunks::<3>().0 {
        let generated = match *value {
            "set" | "true" => true,
            "unset" | "false" => false,
            _ => continue,
        };
        *said.entry(PathBuf::from(path)).or_insert(generated) |= generated;
    }
    Ok(said)
}

#[expect(
    clippy::naive_bytecount,
    reason = "the bytecount crate would be a dependency for one count of newlines"
)]
fn count_lines(path: &Path) -> std::io::Result<(bool, usize)> {
    use std::io::Read;
    if !std::fs::metadata(path)?.is_file() {
        return Err(std::io::Error::other("not a regular file"));
    }
    let mut file = std::fs::File::open(path)?;
    let mut piece = vec![0; 1 << 16];
    let (mut lines, mut last, mut first) = (0, b'\n', true);
    loop {
        let n = file.read(&mut piece)?;
        if n == 0 {
            return Ok((false, lines + usize::from(last != b'\n')));
        }
        if std::mem::take(&mut first) && piece[..n.min(8000)].contains(&0) {
            return Ok((true, 0));
        }
        lines += piece[..n].iter().filter(|&&b| b == b'\n').count();
        last = piece[n - 1];
    }
}

fn parse_name_status(out: &str) -> Vec<ReviewFile> {
    let mut files = Vec::new();
    let mut it = out.split('\0');
    while let Some(head) = it.next().filter(|s| !s.is_empty()) {
        let Some(path) = it.next() else { break };
        let fields: Vec<&str> = head.split(' ').collect();
        let status = fields.last().copied().unwrap_or("M");
        let same_bytes = match fields[..] {
            [_, _, a, b, _] => a == b && b.bytes().any(|c| c != b'0'),
            _ => false,
        };
        let status_char = status.chars().next().unwrap_or('M');
        let old = matches!(status_char, 'R' | 'C').then(|| PathBuf::from(path));
        let path = match &old {
            Some(_) => it.next().unwrap_or(""),
            None => path,
        };
        files.push(ReviewFile {
            path: PathBuf::from(path),
            status: status_char,
            old,
            added: 0,
            deleted: 0,
            binary: false,
            untracked: false,
            generated: false,
            same_bytes,
        });
    }
    files
}

fn parse_numstat(out: &str) -> Vec<(PathBuf, Option<(usize, usize)>)> {
    let mut rows = Vec::new();
    let mut it = out.split('\0');
    while let Some(entry) = it.next().filter(|s| !s.is_empty()) {
        let mut cols = entry.splitn(3, '\t');
        let (Some(a), Some(d), Some(p)) = (cols.next(), cols.next(), cols.next()) else {
            continue;
        };
        let path = if p.is_empty() {
            let _renamed_from = it.next();
            it.next().unwrap_or("")
        } else {
            p
        };
        rows.push((PathBuf::from(path), a.parse().ok().zip(d.parse().ok())));
    }
    rows
}

fn other_remote<'a>(
    git: &dyn Fn(&[&str]) -> Result<String>,
    b: &'a str,
) -> Option<(String, &'a str)> {
    if git(&["rev-parse", "--verify", "-q", &format!("refs/heads/{b}")]).is_ok() {
        return None;
    }
    (git(&["remote"]).ok()?.lines())
        .filter(|r| *r != "origin")
        .find_map(|r| Some((r.to_owned(), b.strip_prefix(r)?.strip_prefix('/')?)))
}

fn fetch<'a>(
    git: &dyn Fn(&[&str]) -> Result<String>,
    b: &'a str,
    base: &str,
) -> Result<(&'a str, bool)> {
    let pushed = b
        .strip_prefix("origin/")
        .filter(|_| git(&["rev-parse", "--verify", "-q", &format!("refs/heads/{b}")]).is_err());
    let b = pushed.unwrap_or(b);
    let mut fetch = vec!["fetch", "-q", "origin", b];
    fetch.extend(base.strip_prefix("origin/"));
    let fetched = git(&fetch).is_ok();
    if !fetched
        && pushed.is_some()
        && git(&["ls-remote", "origin", &format!("refs/heads/{b}")]).is_ok_and(|o| o.is_empty())
    {
        bail!("no branch {b} on origin");
    }
    Ok((b, fetched))
}

fn checkout(
    git: &dyn Fn(&[&str]) -> Result<String>,
    b: &str,
    base: &str,
) -> Result<Option<String>> {
    let (b, fetched) = fetch(git, b, base)?;
    git(&["switch", "-q", b]).with_context(|| format!("switching to {b}"))?;
    let remote = format!("origin/{b}");
    if !fetched {
        return Ok(Some(format!("{remote} not fetched")));
    }
    let current = if own_commits(git, b) {
        git(&["merge-base", "--is-ancestor", &remote, "HEAD"])
    } else {
        git(&["reset", "-q", "--keep", &remote])
    };
    if current.is_ok() {
        return Ok(None);
    }
    Ok(Some(lag(git, &remote, "HEAD")))
}

fn lag(git: &dyn Fn(&[&str]) -> Result<String>, remote: &str, tip: &str) -> String {
    let behind = git(&["merge-base", "--is-ancestor", tip, remote]).is_ok();
    let how = if behind { "behind" } else { "diverged from" };
    format!("{how} {remote}")
}

const FOLDERS: &str = "merl/branches";
const UNOPENED_FOR: Duration = Duration::from_hours(14 * 24);
const DEPENDENCIES: [&str; 3] = [".venv", "node_modules", "target"];

fn branch_of_folder(common: &Path, folder: &Path) -> Option<String> {
    let rel = folder.strip_prefix(common.join(FOLDERS)).ok()?.parent()?;
    rel.to_str().filter(|b| !b.is_empty()).map(str::to_owned)
}

pub fn folder_branch(dirs: &GitDirs) -> Option<String> {
    let gitdir = std::fs::read_to_string(dirs.worktree_head_dir.join("gitdir")).ok()?;
    let folder = Path::new(gitdir.trim_end()).parent()?.canonicalize().ok()?;
    branch_of_folder(&dirs.common_refs_dir, &folder)
}

fn package_modules(folder: &Path) -> Vec<String> {
    let listed = git(folder, &["ls-files", "-z", "--", "*package.json"]).unwrap_or_default();
    (listed.split('\0'))
        .filter_map(|f| f.strip_suffix("/package.json"))
        .map(|d| format!("{d}/node_modules"))
        .collect()
}

fn dependency_links(folder: &Path) -> impl Iterator<Item = String> {
    (DEPENDENCIES.map(String::from).into_iter()).chain(package_modules(folder))
}

fn check_out_folder(root: &Path, b: &str, at: &str) -> Result<PathBuf> {
    if !(Path::new(b).components()).all(|c| matches!(c, std::path::Component::Normal(_))) {
        bail!("no branch {b}");
    }
    let common = dirs(root).context("no git directory")?.common_refs_dir;
    let name = root.file_name().context("no repository name")?;
    let folder = common.join(FOLDERS).join(b).join(name);
    if folder.join(".git").is_file() && dirs(&folder).is_some_and(|d| d.common_refs_dir == common) {
        for name in package_modules(&folder) {
            if std::fs::read_link(folder.join(&name)).is_ok_and(|l| l == root.join(&name)) {
                drop(std::fs::remove_file(folder.join(&name)));
            }
        }
        git(&folder, &["checkout", "-q", "-f", "--detach", at])
            .with_context(|| format!("checking out {b}"))?;
    } else {
        drop(std::fs::remove_dir_all(&folder));
        let path = folder.to_str().context("a folder name that is not UTF-8")?;
        git(root, &["worktree", "add", "-q", "-f", "--detach", path, at])
            .with_context(|| format!("checking out {b}"))?;
    }
    if let Some(d) = dirs(&folder) {
        let admin = d.worktree_head_dir.join("merl");
        drop(
            std::fs::create_dir_all(&admin).and_then(|()| std::fs::write(admin.join("opened"), "")),
        );
    }
    #[cfg(unix)]
    for name in dependency_links(&folder) {
        let (mine, there) = (root.join(&name), folder.join(&name));
        if mine.is_dir() && there.symlink_metadata().is_err() {
            drop(std::os::unix::fs::symlink(mine, there));
        }
    }
    Ok(folder)
}
pub struct Opened(PathBuf);

impl Opened {
    pub fn mark(folder: &Path) -> Option<Self> {
        let dirs = dirs(folder)?;
        folder_branch(&dirs)?;
        let admin = dirs.worktree_head_dir.join("merl");
        let open = admin.join("open");
        let pid = open.join(crate::real::pid().to_string());
        drop(std::fs::create_dir_all(&open).and_then(|()| {
            std::fs::write(admin.join("opened"), "")?;
            std::fs::write(&pid, "")
        }));
        Some(Self(pid))
    }
}

impl Drop for Opened {
    fn drop(&mut self) {
        drop(std::fs::remove_file(&self.0));
    }
}

fn open_in_a_merl(admin: &Path) -> bool {
    let alive = |pid: &str| {
        Command::new("ps")
            .args(["-p", pid])
            .stdout(std::process::Stdio::null())
            .status()
            .map_or(true, |s| s.success())
    };
    std::fs::read_dir(admin.join("merl/open"))
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.file_name().to_str().is_some_and(alive))
}

fn heads_on_origin(root: &Path) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-remote", "--heads", "origin"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env(
            "GIT_SSH_COMMAND",
            std::env::var("GIT_SSH_COMMAND").unwrap_or_else(|_| "ssh".to_owned())
                + " -o BatchMode=yes",
        )
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn sweep_folders(root: &Path, now: SystemTime) {
    let (Some(common), Ok(list)) = (
        dirs(root).map(|d| d.common_refs_dir),
        git(root, &["worktree", "list", "--porcelain"]),
    ) else {
        return;
    };
    let top = common.join(FOLDERS);
    let mut heads = None;
    for folder in (list.lines())
        .filter_map(|l| l.strip_prefix("worktree "))
        .filter_map(|p| Path::new(p).canonicalize().ok())
    {
        let (Some(branch), Some(d)) = (branch_of_folder(&common, &folder), dirs(&folder)) else {
            continue;
        };
        if open_in_a_merl(&d.worktree_head_dir) {
            continue;
        }
        let opened =
            std::fs::metadata(d.worktree_head_dir.join("merl/opened")).and_then(|m| m.modified());
        let aged = opened.map_or(true, |t| {
            now.duration_since(t).unwrap_or_default() > UNOPENED_FOR
        });
        let head = format!("\trefs/heads/{branch}");
        let mut gone = || {
            other_remote(&|args| git(root, args), &branch).is_none()
                && (heads
                    .get_or_insert_with(|| heads_on_origin(root))
                    .as_deref())
                .is_some_and(|h: &str| !h.lines().any(|l| l.ends_with(&head)))
        };
        let Some(path) = folder.to_str() else {
            continue;
        };
        if !(aged || gone()) {
            continue;
        }
        let links: Vec<(PathBuf, PathBuf)> = dependency_links(&folder)
            .filter_map(|name| {
                let link = folder.join(&name);
                let to = std::fs::read_link(&link)
                    .ok()
                    .filter(|l| l.is_absolute() && l.ends_with(&name))?;
                std::fs::remove_file(&link).is_ok().then_some((link, to))
            })
            .collect();
        if git(root, &["worktree", "remove", path]).is_ok() {
            for dir in folder.ancestors().skip(1).take_while(|d| *d != top) {
                if std::fs::remove_dir(dir).is_err() {
                    break;
                }
            }
        } else {
            #[cfg(unix)]
            for (link, to) in links {
                drop(std::os::unix::fs::symlink(to, link));
            }
        }
    }
}

fn own_commits(git: &dyn Fn(&[&str]) -> Result<String>, b: &str) -> bool {
    let (remote, local) = (
        format!("refs/remotes/origin/{b}"),
        format!("refs/heads/{b}"),
    );
    let tips = git(&["reflog", "--format=%H", &remote]).unwrap_or_default();
    let log = git(&["reflog", "--format=%H %gs", &local]).unwrap_or_default();
    let created = log
        .lines()
        .filter_map(|l| l.split_once(' '))
        .filter(|(_, subject)| subject.starts_with("branch: Created from"))
        .map(|(sha, _)| sha);
    let mut args = vec!["rev-list", "--count", &local, "--not", &remote];
    args.extend(tips.lines().chain(created));
    args.push("--");
    !git(&args).is_ok_and(|n| n == "0")
}

pub const BASES: [&str; 5] = [
    "origin/master",
    "origin/main",
    "origin/develop",
    "master",
    "main",
];

fn detect_base(git: &dyn Fn(&[&str]) -> Result<String>) -> Result<String> {
    if let Ok(b) = git(&["symbolic-ref", "-q", "--short", "refs/remotes/origin/HEAD"]) {
        return Ok(b);
    }
    for b in BASES {
        if git(&["rev-parse", "--verify", "-q", &format!("{b}^{{commit}}")]).is_ok() {
            return Ok(b.to_owned());
        }
    }
    bail!("no base branch found: pass --base")
}

#[expect(clippy::too_many_lines, reason = "test tables")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untracked_lines_are_counted_in_64_kib_pieces_and_sniffed_for_nul_in_the_first_8000_bytes() {
        let dir = std::env::temp_dir().join(format!("merl-count-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("log");
        let lines = "x".repeat(99) + "\n";
        std::fs::write(&file, lines.repeat(1000) + "tail").unwrap();
        assert_eq!(count_lines(&file).unwrap(), (false, 1001));
        std::fs::write(&file, "x".repeat(7999) + "\0").unwrap();
        assert_eq!(count_lines(&file).unwrap(), (true, 0));
        std::fs::write(&file, "x".repeat(8000) + "\0").unwrap();
        assert_eq!(count_lines(&file).unwrap(), (false, 1));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hunk_headers_become_marks() {
        let diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n\
                    @@ -1 +1 @@\n-x\n+y\n\
                    @@ -3,0 +4,2 @@\n+a\n+b\n\
                    @@ -8,2 +9,0 @@\n-c\n-d\n";
        let m = parse(diff, false).marks;
        assert_eq!(m[&0], Mark::Changed);
        assert_eq!((m[&3], m[&4]), (Mark::Added, Mark::Added));
        assert_eq!(m[&8], Mark::DeletedBelow);
        assert_eq!(m.len(), 4, "{m:?}");
        assert_eq!(
            parse("@@ -1,3 +0,0 @@\n", false).marks[&0],
            Mark::DeletedBelow,
            "lines deleted above the first line are marked on it"
        );
        assert!(parse("", false).marks.is_empty());
    }

    #[test]
    fn review_keeps_deleted_lines_as_ghosts_and_hunk_starts() {
        use TextLine::{Deleted, File};
        let diff = "@@ -1 +1 @@\n-x\n+y\n\
                    @@ -3,0 +4,2 @@\n+a\n+b\n\
                    @@ -8,2 +9,0 @@\n-c\n-d\n\\ No newline at end of file\n";
        let d = parse(diff, true);
        assert_eq!(d.hunks, vec![File(0), File(3), Deleted(9, 0)]);
        assert_eq!(d.ghosts[&0], vec!["x"]);
        assert_eq!(d.ghosts[&9], vec!["c", "d"]);
        assert_eq!(d.ghosts.get(&3), None);
        assert_eq!(
            d.marks[&0],
            Mark::Added,
            "a changed line is an added one under its ghost: no `Changed` in review"
        );
        assert_eq!(d.marks.len(), 3, "{:?}", d.marks);
        let d = parse("@@ -5 +5 @@\n-a\n+b\n@@ -6,2 +5,0 @@\n-c\n-d\n", true);
        assert_eq!(
            (d.hunks.clone(), d.ghosts[&5].len()),
            (vec![File(4)], 2),
            "a deletion right after a changed last line is one stop, not two"
        );
    }

    #[test]
    fn review_pairs_added_lines_with_the_ghosts_they_rewrite() {
        let diff = "@@ -1,2 +1,2 @@\n-total = price * qty\n-log.debug(\"pricing %s\", item)\n\
                    +total = price * qty * rate\n+log.debug(\"pricing %s\", item)\n\
                    @@ -10 +10,2 @@\n-total = price * qty\n\
                    +log.debug(\"pricing %s\", item)\n+total = price * qty * rate\n";
        let d = parse(diff, true);
        assert_eq!(
            d.ghost_from,
            BTreeMap::from([(0, 0), (9, 9)]),
            "the first ghost at each key is this 0-based base-file line"
        );
        assert_eq!(
            d.pairs,
            HashMap::from([(0, (0, 0)), (1, (0, 1)), (10, (9, 0))]),
            "a balanced hunk pairs in order; an unbalanced one by similarity"
        );
    }

    #[test]
    fn outside_git_there_are_no_marks() {
        let dir = std::env::temp_dir().join(format!("merl-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("f"), "x\n").unwrap();
        assert!(diff(&dir, &dir.join("f"), None, None).marks.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_repository_marks_changed_and_added_lines() {
        let dir = std::env::temp_dir().join(format!("merl-git-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(&dir).args(args).output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&["init", "-q"]);
        std::fs::write(dir.join("f"), "a\nb\nc\n").unwrap();
        git(&["add", "f"]);
        std::fs::write(dir.join("f"), "a\nB\nc\nd\n").unwrap();
        let m = diff(&dir, &dir.join("f"), None, None).marks;
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(m, HashMap::from([(1, Mark::Changed), (3, Mark::Added)]));
    }

    #[test]
    fn nearby_edits_stay_apart_under_inter_hunk_context() {
        let dir = std::env::temp_dir().join(format!("merl-interhunk-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&["init", "-q"]);
        git(&["config", "diff.interHunkContext", "3"]);
        std::fs::write(dir.join("f"), "1\n2\n3\n4\n5\n6\n7\n8\n").unwrap();
        git(&["add", "f"]);
        git(&["commit", "-q", "-m", "base"]);
        std::fs::write(dir.join("f"), "1\n2\nX\n4\n5\nY\n7\n8\n").unwrap();
        let m = diff(&dir, &dir.join("f"), None, None).marks;
        let d = diff(&dir, &dir.join("f"), Some("HEAD"), None);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(m, HashMap::from([(2, Mark::Changed), (5, Mark::Changed)]));
        assert_eq!(d.hunks, vec![TextLine::File(2), TextLine::File(5)]);
        assert_eq!(d.ghosts[&5], vec!["6"]);
    }

    #[test]
    fn a_name_with_glob_characters_is_that_file_alone() {
        let dir = std::env::temp_dir().join(format!("merl-glob-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(dir.join("app")).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&["init", "-q"]);
        for f in ["[id].tsx", "d.tsx", "*.tsx"] {
            std::fs::write(dir.join("app").join(f), "a\nb\n").unwrap();
        }
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        std::fs::write(dir.join("app/d.tsx"), "a\nB\n").unwrap();
        for base in [None, Some("HEAD")] {
            for f in ["app/[id].tsx", "app/*.tsx"] {
                let d = diff(&dir, &dir.join(f), base, None);
                assert_eq!(d, Diff::default(), "{f} against {base:?}");
            }
            let d = diff(&dir, &dir.join("app/d.tsx"), base, None);
            assert_eq!(d.marks.len(), 1, "d.tsx against {base:?}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_changed_line_that_reads_as_a_file_header_is_text_of_its_file() {
        let patch = "diff --git a/q.sql b/q.sql\n--- a/q.sql\n+++ b/q.sql\n\
                     @@ -2 +1,0 @@\n--- a/x.sql\n@@ -4,0 +4 @@\n+++ b/x.sql\n";
        let file = ReviewFile {
            path: PathBuf::from("q.sql"),
            status: 'M',
            old: None,
            added: 0,
            deleted: 1,
            binary: false,
            untracked: false,
            generated: false,
            same_bytes: false,
        };
        let (deleted, added) = changed_lines(Path::new("/nonexistent"), patch, &[file]);
        let rows: Vec<_> = (deleted.iter())
            .map(|d| (d.path.to_str().unwrap(), d.line, d.text.as_str()))
            .collect();
        assert_eq!(rows, [("q.sql", 2, "-- a/x.sql")]);
        let runs: Vec<_> = (added.iter())
            .map(|a| (a.path.to_str().unwrap(), a.line, a.len))
            .collect();
        assert_eq!(runs, [("q.sql", 4, 1)]);
    }

    #[cfg(unix)]
    #[test]
    fn the_base_tree_reads_from_git_what_the_branch_touched_and_links_the_rest() {
        let dir = std::env::temp_dir().join(format!("merl-base-tree-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(dir.join("venv")).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(&dir).args(args).output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join(".gitignore"), "venv/\n").unwrap();
        std::fs::write(dir.join("svc.py"), "X = 1\n").unwrap();
        std::fs::write(dir.join("kept.py"), "K = 1\n").unwrap();
        std::os::unix::fs::symlink("svc.py", dir.join("link.py")).unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "first"]);
        let first = git_out(&dir, "HEAD");
        git(&["commit", "-q", "--allow-empty", "-m", "base"]);
        git(&["switch", "-q", "-c", "feature"]);
        std::fs::write(dir.join("svc.py"), "X = 2\n").unwrap();
        git(&["commit", "-qam", "work"]);
        git(&["rm", "-q", "--cached", "kept.py"]);
        std::fs::write(dir.join("kept.py"), "K = 2\n").unwrap();
        std::fs::write(dir.join("venv/lib.py"), "L = 1\n").unwrap();
        let r = Review::open(&dir, None, Some("main")).unwrap();
        let base = r.base_tree(&dir).unwrap();
        let read = |p: &str| std::fs::read_to_string(base.join(p)).unwrap();
        assert_eq!(read("svc.py"), "X = 1\n");
        assert_eq!(
            read("kept.py"),
            "K = 1\n",
            "untracked since, the base's own"
        );
        assert!(base.join("link.py").is_symlink(), "a link stays a link");
        assert_eq!(
            std::fs::read_link(base.join("venv")).unwrap(),
            dir.join("venv"),
            "an ignored folder is the real one"
        );
        let older = Review {
            merge_base: first,
            ..r
        };
        let other = older.base_tree(&dir).unwrap();
        assert!(other.exists() && !base.exists(), "one base at a time");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn git_out(dir: &Path, rev: &str) -> String {
        git(dir, &["rev-parse", rev]).unwrap()
    }

    #[test]
    fn a_quoted_name_in_a_patch_reads_as_the_name_on_disk() {
        assert_eq!(unquote("a/plain.rs"), "a/plain.rs");
        assert_eq!(unquote(r#""a/x\"y\t\303\244""#), "a/x\"y\t\u{e4}");
    }

    #[test]
    fn a_review_lists_the_branch_files_against_the_merge_base() {
        use TextLine::File;
        let dir = std::env::temp_dir().join(format!("merl-review-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join("src/a.rs"), "a\nb\nc\n").unwrap();
        std::fs::write(dir.join("gone"), "x\n").unwrap();
        let ten = "m1\nm2\nm3\nm4\nm5\nm6\nm7\nm8\nm9\n";
        std::fs::write(dir.join("moved"), format!("{ten}m10\n")).unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        git(&["switch", "-q", "-c", "feature"]);
        std::fs::write(dir.join("src/a.rs"), "a\nB\nc\nd\n").unwrap();
        std::fs::write(dir.join("new"), "n\n").unwrap();
        std::fs::remove_file(dir.join("gone")).unwrap();
        std::fs::rename(dir.join("moved"), dir.join("src/moved.rs")).unwrap();
        std::fs::write(dir.join("src/moved.rs"), format!("{ten}M10\n")).unwrap();
        std::fs::write(dir.join("файл.txt"), "ф\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "work"]);
        git(&["switch", "-q", "main"]);
        std::fs::write(dir.join("other"), "o\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "later"]);
        git(&["switch", "-q", "feature"]);
        git(&["config", "diff.noprefix", "true"]);

        let r = Review::open(&dir, None, None).unwrap();
        assert_eq!((r.branch.as_str(), r.base.as_str()), ("feature", "main"));
        let rows: Vec<_> = r
            .files
            .iter()
            .map(|f| (f.status, f.path.to_str().unwrap(), f.added, f.deleted))
            .collect();
        assert_eq!(
            rows,
            vec![
                ('M', "src/a.rs", 2, 1),
                ('R', "src/moved.rs", 1, 1),
                ('D', "gone", 0, 1),
                ('A', "new", 1, 0),
                ('A', "файл.txt", 1, 0),
            ]
        );
        assert!(
            r.files
                .iter()
                .all(|f| f.status == 'D' || dir.join(&f.path).exists())
        );
        let deleted: Vec<_> = r
            .deleted
            .iter()
            .map(|d| (d.path.to_str().unwrap(), d.line, d.at, d.text.as_str()))
            .collect();
        assert_eq!(
            deleted,
            [
                ("src/a.rs", 2, TextLine::Deleted(1, 0), "b"),
                ("src/moved.rs", 10, TextLine::Deleted(9, 0), "m10"),
                ("gone", 1, File(0), "x"),
            ],
            "the deleted lines (#440), keyed as the review draws them: under the new name of a rename, and in a file the branch deleted, whose text is the base's, as its lines"
        );
        let moved = &r.files[1];
        assert_eq!(
            moved.old.as_deref(),
            Some(Path::new("moved")),
            "the rename diffs against its old name: one hunk, not a whole new file"
        );
        let d = diff(
            &dir,
            &dir.join(&moved.path),
            Some(&r.merge_base),
            moved.old.as_deref(),
        );
        assert_eq!(d.hunks, vec![File(9)]);
        assert_eq!(d.ghosts[&9], vec!["m10"]);
        let d = diff(&dir, &dir.join("src/a.rs"), Some(&r.merge_base), None);
        assert_eq!(d.hunks, vec![File(1), File(3)]);
        assert_eq!(d.ghosts[&1], vec!["b"]);
        assert_eq!(r.base_bytes(&dir, Path::new("gone")).unwrap(), b"x\n");
        assert!(
            Review::open(&dir, None, Some("feature")).is_err(),
            "no changes"
        );
        let real = dir.canonicalize().unwrap();
        let wt = real.with_extension("wt");
        drop(std::fs::remove_dir_all(&wt));
        git(&["worktree", "add", "-q", "-b", "wt", wt.to_str().unwrap()]);
        let common = real.join(".git");
        let same = GitDirs {
            worktree_head_dir: common.clone(),
            common_refs_dir: common.clone(),
        };
        assert_eq!(dirs(&dir), Some(same));
        let GitDirs {
            worktree_head_dir: git_dir,
            common_refs_dir: common_dir,
        } = dirs(&wt).unwrap();
        assert!(git_dir.starts_with(common.join("worktrees")) && git_dir.join("HEAD").exists());
        assert_eq!(common_dir, common);
        std::fs::remove_dir_all(&wt).unwrap();
        git(&["switch", "-q", "main"]);
        let r = Review::open(&dir, Some("feature"), None).unwrap();
        assert_eq!(
            r.branch, "feature",
            "with a branch name the review switches to it; a dirty tree in the way is an error"
        );
        git(&["switch", "-q", "main"]);
        std::fs::write(dir.join("src/a.rs"), "dirty\n").unwrap();
        assert!(
            Review::open(&dir, Some("feature"), None).is_err(),
            "dirty switch"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_branch_checked_out_in_another_worktree_is_reviewed_there() {
        let tmp = std::env::temp_dir().canonicalize().unwrap();
        let dir = tmp.join(format!("merl-other-wt-{}", std::process::id()));
        let wt = dir.with_extension("agent");
        drop(std::fs::remove_dir_all(&dir));
        drop(std::fs::remove_dir_all(&wt));
        std::fs::create_dir_all(&dir).unwrap();
        let git = |at: &Path, args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(at).args(args).output();
            String::from_utf8(out.unwrap().stdout).unwrap()
        };
        git(&dir, &["init", "-q", "-b", "main"]);
        git(&dir, &["config", "user.email", "t@t"]);
        git(&dir, &["config", "user.name", "t"]);
        std::fs::write(dir.join("a.py"), "x = 1\n").unwrap();
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-q", "-m", "init"]);
        git(
            &dir,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "agent/task",
                wt.to_str().unwrap(),
            ],
        );
        std::fs::write(wt.join("a.py"), "x = 2\n").unwrap();
        git(&wt, &["commit", "-q", "-am", "agent work"]);
        std::fs::write(wt.join("new.py"), "y = 3\n").unwrap();

        assert_eq!(worktree_of(&dir, "agent/task"), Some(wt.clone()));
        assert_eq!(
            worktree_of(&dir, "main"),
            None,
            "the branch of this very worktree, or of none, is today's `-r BRANCH`"
        );
        assert_eq!(worktree_of(&dir, "nowhere"), None);
        let r = Review::open(&wt, None, None).unwrap();
        assert_eq!((r.branch.as_str(), r.base.as_str()), ("agent/task", "main"));
        let rows: Vec<_> = r
            .files
            .iter()
            .map(|f| (f.status, f.path.to_str().unwrap()))
            .collect();
        assert_eq!(rows, vec![('M', "a.py"), ('A', "new.py")]);
        let head = |at: &Path| git(at, &["symbolic-ref", "--short", "HEAD"]);
        assert_eq!(
            (head(&dir), head(&wt)),
            ("main\n".into(), "agent/task\n".into()),
            "nothing switched on either side"
        );

        std::fs::remove_dir_all(&wt).unwrap();
        assert_eq!(
            worktree_of(&dir, "agent/task"),
            None,
            "a worktree whose directory is gone is `prunable`: the error stays git's"
        );
        let e = Review::open(&dir, Some("agent/task"), None).unwrap_err();
        assert!(
            format!("{e:#}").contains("switching to agent/task"),
            "{e:#}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_agents_order_sorts_the_review_and_a_refresh_reads_it_again() {
        let dir = std::env::temp_dir().join(format!("merl-order-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(&dir).args(args).output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        git(&["commit", "-q", "--allow-empty", "-m", "base"]);
        git(&["switch", "-q", "-c", "feat/x"]);
        for f in [
            "CHANGELOG.md",
            "docs/a.md",
            "src/api.py",
            "src/types.py",
            "tests/t.py",
        ] {
            std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            std::fs::write(dir.join(f), "x\n").unwrap();
        }
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "change"]);
        let paths = |r: &Review| {
            r.files
                .iter()
                .map(|f| f.path.display().to_string())
                .collect::<Vec<_>>()
        };
        let r = Review::open(&dir, None, Some("main")).unwrap();
        assert_eq!(
            paths(&r),
            [
                "docs/a.md",
                "src/api.py",
                "src/types.py",
                "tests/t.py",
                "CHANGELOG.md"
            ]
        );
        let order = dir.join(".git/merl/review/feat/x");
        std::fs::create_dir_all(order.parent().unwrap()).unwrap();
        std::fs::write(
            &order,
            "# commit abc\nsrc/types.py\ntests/t.py\nsrc/api.py\ngone.py\n",
        )
        .unwrap();
        let r = r.refresh(&dir).unwrap();
        assert_eq!(
            paths(&r),
            [
                "src/types.py",
                "src/api.py",
                "tests/t.py",
                "docs/a.md",
                "CHANGELOG.md"
            ]
        );
        std::fs::write(&order, "CHANGELOG.md\ntests/t.py\n").unwrap();
        let r = r.refresh(&dir).unwrap();
        assert_eq!(
            paths(&r),
            [
                "CHANGELOG.md",
                "tests/t.py",
                "docs/a.md",
                "src/api.py",
                "src/types.py"
            ]
        );
        std::fs::write(dir.join(".git/merl/review/HEAD"), "CHANGELOG.md\n").unwrap();
        git(&["switch", "-q", "--detach"]);
        let r = Review::open(&dir, None, Some("main")).unwrap();
        assert_eq!(
            paths(&r),
            [
                "docs/a.md",
                "src/api.py",
                "src/types.py",
                "tests/t.py",
                "CHANGELOG.md"
            ]
        );
        git(&["switch", "-q", "-c", "other"]);
        let r = Review::open(&dir, None, Some("main")).unwrap();
        assert_eq!(
            paths(&r),
            [
                "docs/a.md",
                "src/api.py",
                "src/types.py",
                "tests/t.py",
                "CHANGELOG.md"
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn untracked_files_are_rows_of_the_review() {
        let dir = std::env::temp_dir().join(format!("merl-untracked-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        let git = |at: &Path, args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(at).args(args).output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&dir, &["init", "-q", "-b", "main"]);
        git(&dir, &["config", "user.email", "t@t"]);
        git(&dir, &["config", "user.name", "t"]);
        std::fs::write(dir.join(".gitignore"), "*.log\n").unwrap();
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-q", "-m", "base"]);
        git(&dir, &["switch", "-q", "-c", "feature"]);
        assert!(Review::open(&dir, None, None).is_err(), "no changes");
        std::fs::write(dir.join(" lead.py"), "x = 1\n").unwrap();
        std::fs::write(dir.join("empty.py"), "").unwrap();
        std::fs::write(dir.join("long.txt"), "ab\n".repeat(3000) + "\0").unwrap();
        std::fs::write(dir.join("logo.png"), b"\x89PNG\0\n\n").unwrap();
        std::fs::write(dir.join("agent.log"), "noise\n").unwrap();
        git(&dir.join("sub"), &["init", "-q"]);
        std::fs::write(dir.join("sub/inner.py"), "y = 2\n").unwrap();
        let r = Review::open(&dir, None, None).unwrap();
        let rows: Vec<_> = r
            .files
            .iter()
            .map(|f| (f.path.to_str().unwrap(), f.added, f.binary, f.has_hunks()))
            .collect();
        assert_eq!(
            rows,
            vec![
                (" lead.py", 1, false, true),
                ("empty.py", 0, false, false),
                ("logo.png", 0, true, false),
                ("long.txt", 3001, false, true),
            ],
            "a name git lists first and in spaces, an empty file, text with a NUL past the 8000 bytes git looks at, a binary, an ignored file, and a nested repository (`sub/`)"
        );
        assert!(r.files.iter().all(|f| f.status == 'A' && f.untracked));
        let d = r.diff(&dir, &dir.join("empty.py"), r.file(Path::new("empty.py")));
        assert_eq!(d, Diff::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn an_untracked_link_to_a_fifo_is_a_row_that_is_not_read() {
        let dir = std::env::temp_dir().join(format!("merl-fifo-link-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(&dir).args(args).output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join("a.py"), "x = 1\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        git(&["switch", "-q", "-c", "feature"]);
        let mkfifo = Command::new("mkfifo").arg(dir.join("pipe")).status();
        assert!(mkfifo.unwrap().success());
        std::os::unix::fs::symlink("pipe", dir.join("pipe.link")).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let at = dir.clone();
        std::thread::spawn(move || {
            let r = Review::open(&at, None, None).unwrap();
            drop(
                tx.send(
                    r.files
                        .iter()
                        .map(|f| (f.path.clone(), f.has_hunks()))
                        .collect(),
                ),
            );
        });
        let rows: Vec<(PathBuf, bool)> = rx.recv_timeout(Duration::from_secs(60)).unwrap();
        assert_eq!(rows, [(PathBuf::from("pipe.link"), false)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_review_opens_on_a_file_with_text_a_binary_or_none() {
        let dir = std::env::temp_dir().join(format!("merl-first-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(&dir).args(args).output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join("docs/a.md"), "hello\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        git(&["switch", "-q", "-c", "feature"]);
        std::os::unix::fs::symlink("docs", dir.join("alink")).unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "link"]);
        let first = |dir: &Path| {
            Review::open(dir, None, None)
                .unwrap()
                .first_file(dir, false)
        };
        assert_eq!(first(&dir), None);
        std::fs::write(dir.join("logo.png"), b"\x89PNG\0\n").unwrap();
        assert_eq!(first(&dir), Some(dir.join("logo.png")));
        std::fs::write(dir.join("z.py"), "x = 1\n").unwrap();
        assert_eq!(first(&dir), Some(dir.join("z.py")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_named_review_reads_what_was_pushed() {
        let dir = std::env::temp_dir().join(format!("merl-pushed-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let git = |at: &str, args: &[&str]| {
            let out = Command::new("git")
                .current_dir(dir.join(at))
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        let commit = |at: &str, file: &str| {
            std::fs::write(dir.join(at).join(file), format!("{file}\n")).unwrap();
            git(at, &["add", "."]);
            git(at, &["commit", "-q", "-m", file]);
        };
        let push = || git("a", &["push", "-q", "-f", "origin", "HEAD"]);
        let open = || Review::open(&dir.join("b"), Some("feat"), None).unwrap();
        let names = |r: &Review| {
            let names = r
                .files
                .iter()
                .map(|f| f.path.to_string_lossy().into_owned());
            names.collect::<Vec<_>>()
        };
        git("", &["init", "-q", "--bare", "-b", "main", "remote.git"]);
        git("", &["init", "-q", "-b", "main", "a"]);
        let remote = dir.join("remote.git");
        git("a", &["remote", "add", "origin", remote.to_str().unwrap()]);
        commit("a", "README");
        push();
        git("a", &["switch", "-q", "-c", "feat"]);
        commit("a", "one");
        push();
        git("", &["clone", "-q", remote.to_str().unwrap(), "b"]);
        let r = open();
        assert_eq!((names(&r), r.note), (vec!["one".to_owned()], None));
        git("b", &["switch", "-q", "main"]);

        git("a", &["reset", "-q", "--hard", "HEAD~1"]);
        commit("a", "two");
        push();
        assert_eq!(
            names(&open()),
            ["two"],
            "rebased and force-pushed: a copy nobody committed on takes the new history, though `git clone` left no trace of the tip it was created from"
        );
        git("b", &["switch", "-q", "main"]);

        commit("a", "three");
        push();
        let r = open();
        assert_eq!(
            names(&r),
            ["three", "two"],
            "pushed since: the local branch from the last review moves on to it"
        );
        assert_eq!(r.note, None);

        git("a", &["switch", "-q", "main"]);
        commit("a", "later");
        push();
        git("a", &["switch", "-q", "feat"]);
        git("a", &["merge", "-q", "--no-edit", "main"]);
        push();
        assert_eq!(
            names(&open()),
            ["three", "two"],
            "the branch took in a base that moved on: that commit is the base's, not the branch's, though this clone never fetched it"
        );

        std::fs::write(dir.join("b/two"), "mine\n").unwrap();
        std::fs::write(dir.join("a/two"), "fixed\n").unwrap();
        git("a", &["commit", "-q", "-am", "fix"]);
        push();
        assert_eq!(
            open().note.as_deref(),
            Some("behind origin/feat"),
            "local changes in the way: the branch stays, and the review says it is behind"
        );
        git("b", &["checkout", "-q", "--", "two"]);
        assert_eq!(open().note, None);

        commit("b", "mine");
        let r = open();
        assert_eq!(
            (names(&r).contains(&"mine".into()), r.note),
            (true, None),
            "the reviewer's own commit stays, ahead of what was pushed or diverged from it"
        );
        commit("a", "four");
        push();
        let r = open();
        assert_eq!(names(&r), ["mine", "three", "two"]);
        assert_eq!(r.note.as_deref(), Some("diverged from origin/feat"));

        git("b", &["reset", "-q", "--hard", "origin/feat~1"]);
        std::fs::remove_dir_all(&remote).unwrap();
        let r = open();
        assert_eq!(
            names(&r),
            ["three", "two"],
            "nothing to fetch from: the branch as it is, and the review says so"
        );
        assert_eq!(r.note.as_deref(), Some("origin/feat not fetched"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_review_of_remote_slash_branch_reads_the_pushed_branch_and_moves_no_local_one() {
        let dir = std::env::temp_dir().join(format!("merl-origin-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let dir = dir.canonicalize().unwrap();
        let git = |at: &str, args: &[&str]| {
            let out = Command::new("git")
                .current_dir(dir.join(at))
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}");
            String::from_utf8(out.stdout).unwrap()
        };
        let commit = |at: &str, file: &str| {
            std::fs::write(dir.join(at).join(file), format!("{file}\n")).unwrap();
            git(at, &["add", "."]);
            git(at, &["commit", "-q", "-m", file]);
        };
        let b = dir.join("b");
        let open = |branch: &str| Review::open_branch(&b, branch, None).map(|(_, r)| r);
        let error = |branch: &str| format!("{:#}", open(branch).unwrap_err());
        let branches = || git("b", &["branch", "--format=%(refname:short) %(objectname)"]);
        git("", &["init", "-q", "--bare", "-b", "main", "remote.git"]);
        git("", &["init", "-q", "--bare", "-b", "main", "fork.git"]);
        git("", &["clone", "-q", "remote.git", "a"]);
        git("a", &["switch", "-q", "-c", "main"]);
        commit("a", "README");
        git("a", &["push", "-q", "origin", "main"]);
        git("", &["clone", "-q", "remote.git", "b"]);
        git("b", &["remote", "add", "upstream", "../fork.git"]);
        git("a", &["switch", "-q", "-c", "feat"]);
        commit("a", "one");
        git("a", &["push", "-q", "origin", "feat"]);
        git("a", &["switch", "-q", "-c", "theirs", "main"]);
        commit("a", "two");
        git("a", &["push", "-q", "../fork.git", "theirs:feat"]);
        let before = branches();

        let r = open("origin/feat").unwrap();
        assert_eq!(
            (r.branch.as_str(), r.note, r.files[0].path.to_str()),
            ("feat", None, Some("one")),
            "fetched and read in its folder"
        );
        let (folder, r) = Review::open_branch(&b, "upstream/feat", None).unwrap();
        assert_eq!(
            (r.branch.as_str(), r.note, r.files[0].path.to_str()),
            ("upstream/feat", None, Some("two")),
            "another remote's branch, fetched from that remote and read against origin's base"
        );
        assert_eq!(
            branches(),
            before,
            "no local branch made or moved by either"
        );
        sweep_folders(&b, SystemTime::now());
        assert!(
            folder.is_dir(),
            "a branch of another remote is not gone because origin lacks it"
        );
        git("a", &["switch", "-q", "-c", "solo", "main"]);
        commit("a", "three");
        git(
            "a",
            &[
                "push",
                "-q",
                "origin",
                "solo:upstream/solo",
                "solo:upstream/feat2",
            ],
        );
        git("a", &["switch", "-q", "-c", "fork", "main"]);
        commit("a", "four");
        git("a", &["push", "-q", "../fork.git", "fork:feat2"]);
        for branch in ["upstream/solo", "upstream/feat2"] {
            let r = open(branch).unwrap();
            assert_eq!(
                (r.branch.as_str(), r.files[0].path.to_str()),
                (branch, Some("three")),
                "origin's branch of the whole name wins over the remote `upstream`"
            );
        }

        git("b", &["branch", "mine"]);
        assert_eq!(
            error("origin/mine"),
            "no branch mine on origin",
            "not on origin: an error, though a local branch of that name exists"
        );
        assert_eq!(error("nowhere"), "no branch nowhere on origin");
        assert_eq!(error("upstream/nowhere"), "no branch nowhere on upstream");

        git("b", &["switch", "-q", "-c", "origin/lit"]);
        commit("b", "lit");
        git("b", &["switch", "-q", "main"]);
        assert_eq!(
            open("origin/lit").unwrap().branch,
            "origin/lit",
            "a local branch literally named `origin/lit` is that branch"
        );

        std::fs::remove_dir_all(dir.join("remote.git")).unwrap();
        std::fs::remove_dir_all(dir.join("fork.git")).unwrap();
        let notes = ["origin/feat", "upstream/feat"].map(|b| open(b).unwrap().note);
        assert_eq!(
            notes,
            [
                Some("origin/feat not fetched".to_owned()),
                Some("upstream/feat not fetched".to_owned())
            ],
            "unreachable: what was fetched last, and the review says so"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn changed_files_are_m_and_new_ones_a_under_the_root() {
        let dir = std::env::temp_dir().join(format!("merl-marks-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(dir.join("sub/deep")).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(&dir).args(args).output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        for f in [
            "top.rs",
            "sub/edited.rs",
            "sub/staged.rs",
            "sub/same.rs",
            "sub/moved.rs",
        ] {
            std::fs::write(dir.join(f), "one\n").unwrap();
        }
        std::fs::write(dir.join(".gitignore"), "*.log\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        assert!(changed_files(&dir.join("sub")).is_empty());

        for f in ["top.rs", "sub/edited.rs", "sub/staged.rs"] {
            std::fs::write(dir.join(f), "one\ntwo\n").unwrap();
        }
        for f in ["sub/new.rs", "sub/deep/untracked.rs", "sub/build.log"] {
            std::fs::write(dir.join(f), "new\n").unwrap();
        }
        git(&["add", "sub/staged.rs", "sub/new.rs"]);
        git(&["mv", "sub/moved.rs", "sub/renamed.rs"]);
        let mut got: Vec<(String, char)> = changed_files(&dir.join("sub"))
            .into_iter()
            .map(|(p, m)| (p.to_string_lossy().into_owned(), m))
            .collect();
        got.sort();
        let want = [
            ("deep/untracked.rs", 'A'),
            ("edited.rs", 'M'),
            ("moved.rs", 'M'),
            ("new.rs", 'A'),
            ("renamed.rs", 'A'),
            ("staged.rs", 'M'),
        ];
        assert_eq!(got, want.map(|(p, m)| (p.to_owned(), m)));
        assert_eq!(changed_files(&dir)[Path::new("top.rs")], 'M');
        assert!(changed_files(&std::env::temp_dir()).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_new_file_over_a_staged_deletion_is_changed() {
        let marks = parse_status("D  a.rs\0?? a.rs\0 M b.rs\0AM c.rs\0", Path::new(""));
        assert_eq!(marks[Path::new("a.rs")], 'M');
        assert_eq!(marks[Path::new("b.rs")], 'M');
        assert_eq!(marks[Path::new("c.rs")], 'A');
    }

    #[test]
    fn a_conflicted_file_is_changed_when_head_has_it() {
        let out = "AA both\0AU ours\0UU edited\0UD kept\0UA theirs\0DU revived\0 A intent\0";
        let marks = parse_status(out, Path::new(""));
        let letter = |p: &str| marks[Path::new(p)];
        let got = [
            "both", "ours", "edited", "kept", "theirs", "revived", "intent",
        ]
        .map(letter);
        assert_eq!(got, ['M', 'M', 'M', 'M', 'A', 'A', 'A']);
    }

    #[test]
    fn nul_separated_listings_carry_renames_and_binaries() {
        let f = parse_name_status("M\0a.rs\0R090\0old.rs\0new.rs\0A\0файл\0");
        let rows: Vec<_> = f
            .iter()
            .map(|f| (f.status, f.path.to_str().unwrap(), f.old.as_deref()))
            .collect();
        assert_eq!(
            rows,
            vec![
                ('M', "a.rs", None),
                ('R', "new.rs", Some(Path::new("old.rs"))),
                ('A', "файл", None)
            ]
        );
        let n = parse_numstat("1\t2\ta.rs\0-\t-\tbin\x003\t4\t\0old.rs\0new.rs\0");
        assert_eq!(
            n,
            vec![
                ("a.rs".into(), Some((1, 2))),
                ("bin".into(), None),
                ("new.rs".into(), Some((3, 4)))
            ]
        );
    }

    #[test]
    fn generated_files_are_known_by_name_attribute_and_header() {
        let dir = std::env::temp_dir().join(format!("merl-generated-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(dir.join("gen")).unwrap();
        let git = |args: &[&str]| {
            let out = Command::new("git").arg("-C").arg(&dir).args(args).output();
            assert!(out.unwrap().status.success(), "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join("base"), "b\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        git(&["switch", "-q", "-c", "feature"]);
        let attrs = "gen/** linguist-generated\nlab.txt gitlab-generated\n\
                     no.txt linguist-generated=false\nunset.txt -linguist-generated\n\
                     open/poetry.lock -linguist-generated\n\
                     open/package-lock.json linguist-generated=false\n";
        let header = "// Code generated by protoc-gen-go. DO NOT EDIT.\n";
        let late = format!("{}{header}", "//\n".repeat(40));
        let within = format!("{}{header}", "//\n".repeat(39));
        let files: &[(&str, &str, bool)] = &[
            (".gitattributes", attrs, false),
            ("web/package-lock.json", "{}\n", true),
            ("Cargo.lock", "# @generated\n", true),
            ("poetry.lock", "x\n", true),
            ("infra/.terraform.lock.hcl", "x\n", true),
            ("static/app.min.js", "x\n", true),
            ("static/app.js.map", "{}\n", true),
            ("web/node_modules/left-pad/index.js", "x\n", true),
            ("gen/client.ts", "export {}\n", true),
            ("lab.txt", "l\n", true),
            ("api.pb.go", header, true),
            ("within.go", &within, true),
            ("yarn.lock", "x\n", false),
            ("go.sum", "x\n", false),
            ("Gemfile.lock", "x\n", false),
            ("open/poetry.lock", "x\n", false),
            ("open/package-lock.json", "{}\n", false),
            ("no.txt", "n\n", false),
            ("unset.txt", "u\n", false),
            (
                "schema.js",
                "/**\n * @generated SignedSource<<1>>\n */\n",
                false,
            ),
            ("Cargo.lock.bak", "x\n", false),
            ("mylock.json", "x\n", false),
            ("late.go", &late, false),
            ("half.go", "// Code generated by hand, edit away.\n", false),
            ("notgo.ts", header, false),
            ("plain.rs", "fn main() {}\n", false),
        ];
        for (name, text, _) in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        git(&["add", "-A", "-f"]);
        git(&["commit", "-q", "-m", "work"]);
        std::fs::write(dir.join("gen/new.ts"), "n\n").unwrap();
        std::fs::write(dir.join("new.go"), header).unwrap();
        let r = Review::open(&dir, None, None).unwrap();
        drop(std::fs::remove_dir_all(&dir));
        let mut want: Vec<_> = files.iter().map(|&(n, _, g)| (n, g)).collect();
        want.extend([("gen/new.ts", true), ("new.go", true)]);
        want.sort_unstable();
        let mut got: Vec<_> = (r.files.iter())
            .map(|f| (f.path.to_str().unwrap(), f.generated))
            .collect();
        got.sort_unstable();
        assert_eq!(got, want, "untracked: read the same way");
    }
}
