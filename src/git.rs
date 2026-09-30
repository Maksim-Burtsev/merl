//! Gutter marks: which lines differ from the git index, as VS Code's editor gutter shows them.
//! No diff of our own: `git diff -U0` is run after every save and reload, and only its hunk
//! headers are read. In review mode the same diff is taken against the branch's base, and the
//! deleted lines are kept too, to be drawn as ghosts.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Added,
    Changed,
    /// Lines were removed right below this one.
    DeletedBelow,
}

/// A line of a file as a review draws it (#439): a line of the file, or the `i`th of the lines
/// the branch deleted above file line `key`, `Diff::ghosts[key][i]` (`key` is `lines.len()` for
/// those deleted at the end). Ordered as drawn: the deleted lines at a key come before the
/// file's line there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextLine {
    File(usize),
    Deleted(usize, usize),
}

impl TextLine {
    /// The file line: this one's, or the one a deleted line is drawn above.
    pub fn key(self) -> usize {
        match self {
            TextLine::File(l) | TextLine::Deleted(l, _) => l,
        }
    }
}

impl Ord for TextLine {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let order = |t: &TextLine| match *t {
            TextLine::File(l) => (l, usize::MAX),
            TextLine::Deleted(k, i) => (k, i),
        };
        order(self).cmp(&order(other))
    }
}

impl PartialOrd for TextLine {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// What one file's diff looks like from the editor: marks per 0-based line, and, in review
/// mode, the deleted text as ghost lines keyed by the line they sit above (`lines.len()` for
/// the end of the file) plus the first line of every hunk, for `c` / `C`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    pub marks: HashMap<usize, Mark>,
    pub ghosts: BTreeMap<usize, Vec<String>>,
    /// Review: the first line of every hunk, deleted when the hunk starts with a deletion.
    pub hunks: Vec<TextLine>,
    /// Review: the 0-based base-file line of the first ghost at each key; the ghosts under one
    /// key are consecutive base lines.
    pub ghost_from: BTreeMap<usize, usize>,
    /// Review: an added line (0-based line of the working file) → `(ghost key, index in that
    /// key's ghosts)` of the deleted line it pairs with, from `intraline::pair` per hunk.
    pub pairs: HashMap<usize, (usize, usize)>,
}

/// The diff of `path` in the working tree against the index, or against `base` when given
/// (review mode: ghosts and hunks are only kept then). `old` is the name the file had at the
/// base when the branch renamed it: with both names in the pathspec git pairs them.
/// `--inter-hunk-context=0` beats a user's `diff.interHunkContext`, which would join nearby
/// edits into one hunk with the unchanged lines between them.
pub fn diff(root: &Path, path: &Path, base: Option<&str>, old: Option<&Path>) -> Diff {
    let mut cmd = Command::new("git");
    // The names are files, not patterns: `[id].tsx` is not `d.tsx` too.
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

/// Reads `@@ -a,b +c,d @@` headers; `b` and `d` default to 1 when absent. With `review`, the
/// `-` lines under a header become ghosts and a changed line is just an added one under them.
fn parse(diff: &str, review: bool) -> Diff {
    let mut out = Diff::default();
    let mut lines = diff.lines().peekable();
    // Review: the key right after the last hunk's lines. A hunk that starts there reads on from
    // it, with no line between them, and is no stop of its own.
    let mut after = None;
    while let Some(line) = lines.next() {
        if !line.starts_with("@@ ") {
            continue;
        }
        let Some((old, new)) = line[3..].split_once(" +") else {
            continue;
        };
        let (Some((old_start, old_n)), Some((new_start, new_n))) = (
            range(old.trim_start_matches('-')),
            range(new.split(' ').next().unwrap_or("")),
        ) else {
            continue;
        };
        // 0-based line the hunk's new text starts on; a pure deletion sits above `new_start + 1`.
        let at = if new_n == 0 { new_start } else { new_start - 1 };
        if review {
            let mut deleted = Vec::new();
            let mut added = Vec::new();
            while let Some(l) =
                lines.next_if(|l| l.starts_with('-') || l.starts_with('+') || l.starts_with('\\'))
            {
                if let Some(text) = l.strip_prefix('-') {
                    deleted.push(text.to_string());
                } else if let Some(text) = l.strip_prefix('+') {
                    added.push(text.to_string());
                }
            }
            let mut first = TextLine::File(at);
            if !deleted.is_empty() {
                let ghosts = out.ghosts.entry(at).or_default();
                let offset = ghosts.len();
                first = TextLine::Deleted(at, offset);
                // `a` of `-a,b` is 1-based; a hunk with deleted lines never has `a == 0`.
                out.ghost_from.entry(at).or_insert(old_start - 1);
                for (i, j) in crate::intraline::pair(&deleted, &added) {
                    out.pairs.insert(at + j, (at, offset + i));
                }
                ghosts.extend(deleted);
            }
            if after != Some(at) {
                out.hunks.push(first);
            }
            after = Some(at + new_n);
            for l in at..at + new_n {
                out.marks.insert(l, Mark::Added);
            }
            continue;
        }
        if new_n == 0 {
            // A pure deletion after 1-based line `new_start`; after "line 0" means above line 1.
            out.marks
                .insert(new_start.saturating_sub(1), Mark::DeletedBelow);
            continue;
        }
        let mark = if old_n == 0 {
            Mark::Added
        } else {
            Mark::Changed
        };
        for l in new_start - 1..new_start - 1 + new_n {
            out.marks.insert(l, mark);
        }
    }
    out
}

/// `start,count` or `start` → (start, count).
fn range(s: &str) -> Option<(usize, usize)> {
    match s.split_once(',') {
        Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
        None => Some((s.parse().ok()?, 1)),
    }
}

// ---- review mode -----------------------------------------------------------

/// One file of the branch under review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFile {
    /// Relative to the root.
    pub path: PathBuf,
    /// `M`, `A`, `D`, `R`...: the first letter of `git diff --name-status`.
    pub status: char,
    /// The name at the base, for a rename.
    pub old: Option<PathBuf>,
    pub added: usize,
    pub deleted: usize,
    /// `git diff --numstat` counts `-` for it.
    pub binary: bool,
    /// Not in git yet: listed as added, and its diff is the whole file.
    pub untracked: bool,
    /// Generated as both GitHub and GitLab have it (see [`generated`]): the review folds it until
    /// Enter loads its diff (#243).
    pub generated: bool,
}

impl ReviewFile {
    /// Is there a line to read? Not in a binary file, a mode change or a pure rename: `c` walks
    /// past those, and the panel still opens them.
    pub fn has_hunks(&self) -> bool {
        self.added + self.deleted > 0
    }
}

/// `merl --review`: the checked-out branch against its base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    pub branch: String,
    pub base: String,
    /// The merge base commit: what every diff and the file list are taken against, so the
    /// working tree lines up with the numbers.
    pub merge_base: String,
    pub files: Vec<ReviewFile>,
    /// What opening found about the branch against `origin` (`diverged from origin/feat`), for
    /// the status bar; `App::start_review` takes it, so it is said once.
    pub note: Option<String>,
}

impl Review {
    /// Finds the base, checks out `branch` when given, as `origin` has it (see `checkout`), and
    /// lists the files.
    pub fn open(root: &Path, branch: Option<&str>, base: Option<&str>) -> Result<Self> {
        let git = |args: &[&str]| git(root, args);
        let base = match base {
            Some(b) => b.to_string(),
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

    /// The file the review opens on when none is named: the first with something to read; a
    /// branch of binaries opens on one, and one of submodules and links to directories on none
    /// (#404). A file the branch deleted is not on disk to open.
    pub fn first_file(&self, root: &Path) -> Option<PathBuf> {
        let on_disk = || self.files.iter().filter(|f| f.status != 'D');
        on_disk()
            .find(|f| f.has_hunks())
            .map(|f| root.join(&f.path))
            .or_else(|| on_disk().map(|f| root.join(&f.path)).find(|p| p.is_file()))
    }

    /// The same review as the branch and the working tree are now: after a commit, an edit, a
    /// new file. Nothing is fetched or switched, and an empty list is an answer.
    pub fn refresh(&self, root: &Path) -> Result<Self> {
        Self::list(root, self.base.clone())
    }

    fn list(root: &Path, base: String) -> Result<Self> {
        let git = |args: &[&str]| git(root, args);
        // The one reading of HEAD, which the viewed marks are kept by too: the branch's own name,
        // which a tag of the same name does not shadow; during a rebase, the branch being rebased,
        // as git records it; otherwise `HEAD`, detached (no branch may be named so).
        let head = git(&["symbolic-ref", "-q", "HEAD"]).or_else(|_| rebasing(&git, root));
        let branch = (head.as_deref().ok())
            .and_then(|h| h.strip_prefix("refs/heads/"))
            .unwrap_or("HEAD")
            .to_string();
        let merge_base = git(&["merge-base", &base, "HEAD"])
            .with_context(|| format!("no merge base between {base} and HEAD"))?;
        // `-z`: NUL-separated and unquoted, so a non-ASCII name is the name on disk.
        let mut files = parse_name_status(&git(&["diff", "--name-status", "-z", &merge_base])?);
        // A submodule counts as one line (`Subproject commit …`) that is no text to read: left
        // out here it keeps its row in the panel and, like a pure rename, is not a stop.
        let numstat = [
            "diff",
            "--numstat",
            "--ignore-submodules",
            "-z",
            &merge_base,
        ];
        // A symlink counts one line too, where it points: to a directory, no text either (#404).
        let dir_link = |p: &Path| p.is_symlink() && p.is_dir();
        for (path, counts) in parse_numstat(&git(&numstat)?) {
            if let Some(f) = files.iter_mut().find(|f| f.path == path)
                && !dir_link(&root.join(&path))
            {
                f.binary = counts.is_none();
                (f.added, f.deleted) = counts.unwrap_or((0, 0));
            }
        }
        // `git diff` does not list untracked files, and a new module is the first thing to
        // review. A nested repository is listed as `dir/`: not a file to read. A path the
        // branch deleted and somebody wrote again (or `git rm --cached`) is in both listings:
        // one row per path, and the file on disk is the one to read.
        let others = git(&["ls-files", "--others", "--exclude-standard", "-z"])?;
        for path in others
            .split('\0')
            .filter(|p| !p.is_empty() && !p.ends_with('/'))
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
        // The panel's order, so `c` walks the files top to bottom.
        files.sort_by_cached_key(|f| crate::tree::sort_key(&f.path, false));
        Ok(Self {
            branch,
            base,
            merge_base,
            files,
            note: None,
        })
    }

    /// The marks, ghosts and hunks of one file of the working tree against the merge base;
    /// `file` is its row, when it has one. An untracked file is one hunk of added lines.
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

    /// The branch under review; `None` on a detached HEAD, which `branch` shows as `HEAD`.
    pub fn branch_name(&self) -> Option<&str> {
        (self.branch != "HEAD").then_some(self.branch.as_str())
    }

    /// The branch, or the short commit of a detached HEAD: the review stats count the rounds of
    /// each, and every detached review is not one branch called `HEAD`.
    pub fn branch_or_commit(&self, root: &Path) -> String {
        match self.branch.as_str() {
            "HEAD" => git(root, &["rev-parse", "--short", "HEAD"]).unwrap_or(self.branch.clone()),
            branch => branch.to_string(),
        }
    }

    pub fn file(&self, rel: &Path) -> Option<&ReviewFile> {
        self.files.iter().find(|f| f.path == rel)
    }

    /// The content of `rel` at the base: what a deleted file looked like.
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
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
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
    // Only the newline git ends a line with: a `-z` listing may start with a name in spaces.
    Ok(String::from_utf8_lossy(&out.stdout)
        .trim_end_matches('\n')
        .to_string())
}

/// The git directory of the worktree at `root` (where `HEAD` is) and the repository's common
/// one (where the refs are), canonical. They are the same `.git` outside a linked worktree.
pub fn dirs(root: &Path) -> Option<(PathBuf, PathBuf)> {
    let out = git(root, &["rev-parse", "--git-dir", "--git-common-dir"]).ok()?;
    let mut dirs = out.lines().map(|d| root.join(d).canonicalize().ok());
    Some((dirs.next()??, dirs.next()??))
}

/// The other worktree of this repository that has `branch` checked out, an agent's say (#396):
/// `-r BRANCH` reviews it there, since git will not check the branch out twice. A worktree git
/// calls `prunable` (its directory is gone) is not one.
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

/// The branch a rebase (stopped on a conflict, say) is rebasing: `refs/heads/…`, from the
/// `head-name` git keeps in the worktree's git dir.
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
        .map(|name| name.trim().to_string())
        .context("no rebase in progress")
}

/// The row of an untracked file: `A`, every line added. Binary is what git calls binary, a NUL
/// in the first 8000 bytes.
///
/// ponytail: every untracked text file is read on every refresh to count its lines, in 64 KiB
/// pieces, so a huge log costs time and no memory. A branch with thousands of them, or a
/// gigabyte of text, wants the counts cached by size and mtime.
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
    }
}

/// What both GitHub and GitLab fold as generated, by the file's exact name: the lock files
/// of Linguist's `generated.rb` that go-enry's `generated.go` also has (#243). `yarn.lock`,
/// `go.sum` and `Gemfile.lock` are in neither, and stay open.
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

/// Ends of names both forges fold: minified scripts and styles, and their source maps.
const GENERATED_ENDS: &[&str] = &[".min.js", ".min.css", ".js.map", ".css.map"];

/// Directories both forges fold whatever is in them.
const GENERATED_DIRS: &[&str] = &["node_modules", "Godeps"];

/// Is `f` generated as both forges have it, by its path or, for Go, its first 40 lines: Go's
/// `// Code generated … DO NOT EDIT.` Certain only, no guessing by content (#243).
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

/// What `.gitattributes` says of `files` with `linguist-generated` or `gitlab-generated`, the
/// attributes the forges fold a file's diff by: `true` for a path it marks generated, `false`
/// for one it marks not generated (`-linguist-generated`, `=false`), which then stays open
/// whatever its name.
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
    // Written from a thread: a long list would fill the pipe back while git fills stdout.
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
        // Either attribute marking it generated folds it.
        *said.entry(PathBuf::from(path)).or_insert(generated) |= generated;
    }
    Ok(said)
}

/// Is the file binary, and its lines as `git diff --numstat` counts them: a last line without
/// a newline is a line. A binary file is not read past the 8000 bytes that say so.
fn count_lines(path: &Path) -> std::io::Result<(bool, usize)> {
    use std::io::Read;
    // An untracked link to a FIFO would hold the open until something writes to it (#405).
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

/// `git diff --name-status -z`: `STATUS\0path\0`, and `Rnnn\0old\0new\0` for a rename.
fn parse_name_status(out: &str) -> Vec<ReviewFile> {
    let mut files = Vec::new();
    let mut it = out.split('\0');
    while let Some(status) = it.next().filter(|s| !s.is_empty()) {
        let Some(path) = it.next() else { break };
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
        });
    }
    files
}

/// `git diff --numstat -z`: `added\tdeleted\tpath\0`, and `added\tdeleted\t\0old\0new\0` for a
/// rename. Binary files count `-`: no counts.
fn parse_numstat(out: &str) -> Vec<(PathBuf, Option<(usize, usize)>)> {
    let mut rows = Vec::new();
    let mut it = out.split('\0');
    while let Some(entry) = it.next().filter(|s| !s.is_empty()) {
        let mut cols = entry.splitn(3, '\t');
        let (Some(a), Some(d), Some(p)) = (cols.next(), cols.next(), cols.next()) else {
            continue;
        };
        let path = if p.is_empty() {
            // Rename: old, then new, in the next two fields.
            it.next();
            it.next().unwrap_or("")
        } else {
            p
        };
        rows.push((PathBuf::from(path), a.parse().ok().zip(d.parse().ok())));
    }
    rows
}

/// `--review BRANCH` reads what the merge request shows: the branch and the base are fetched in
/// one go, and the local branch is brought to what was pushed. The reviewer's own commits are
/// never rewritten, and local changes are never lost; the answer is then what the status bar
/// says instead (`diverged from origin/feat`).
fn checkout(
    git: &dyn Fn(&[&str]) -> Result<String>,
    b: &str,
    base: &str,
) -> Result<Option<String>> {
    // `origin/feat`, copied from `git branch -a` or a merge request, is `feat` as it was pushed
    // (#271), which origin must then have; a local branch literally named so is that branch, as
    // `git switch` takes it. Other remotes stay local names: the fetch and the base are origin's.
    let pushed = b
        .strip_prefix("origin/")
        .filter(|_| git(&["rev-parse", "--verify", "-q", &format!("refs/heads/{b}")]).is_err());
    let b = pushed.unwrap_or(b);
    let mut fetch = vec!["fetch", "-q", "origin", b];
    fetch.extend(base.strip_prefix("origin/"));
    let fetched = git(&fetch).is_ok();
    // A failed fetch is origin offline, or origin without the branch: `ls-remote` answers only
    // when origin is reachable, and then with nothing.
    if !fetched
        && pushed.is_some()
        && git(&["ls-remote", "origin", &format!("refs/heads/{b}")]).is_ok_and(|o| o.is_empty())
    {
        bail!("no branch {b} on origin");
    }
    git(&["switch", "-q", b]).with_context(|| format!("switching to {b}"))?;
    let remote = format!("origin/{b}");
    if !fetched {
        return Ok(Some(format!("{remote} not fetched")));
    }
    // A copy nobody committed on takes what was pushed, after a force-push too, unless local
    // changes are in the way (`--keep`). With commits of one's own it is up to date only ahead.
    let current = if own_commits(git, b) {
        git(&["merge-base", "--is-ancestor", &remote, "HEAD"])
    } else {
        git(&["reset", "-q", "--keep", &remote])
    };
    if current.is_ok() {
        return Ok(None);
    }
    let behind = git(&["merge-base", "--is-ancestor", "HEAD", &remote]).is_ok();
    let how = if behind { "behind" } else { "diverged from" };
    Ok(Some(format!("{how} {remote}")))
}

/// Does the checked-out branch `b` hold commits `origin/b` never pointed at in this repository?
/// The remote ref's reflog has each tip a fetch or a push brought, but not the one `git clone`
/// did, so the commit the local branch was created from counts too. An expired reflog only
/// makes more commits look like one's own, and the branch then stays.
fn own_commits(git: &dyn Fn(&[&str]) -> Result<String>, b: &str) -> bool {
    let remote = format!("refs/remotes/origin/{b}");
    let tips = git(&["reflog", "--format=%H", &remote]).unwrap_or_default();
    let log = git(&["reflog", "--format=%H %gs", &format!("refs/heads/{b}")]).unwrap_or_default();
    let created = log
        .lines()
        .filter_map(|l| l.split_once(' '))
        .filter(|(_, subject)| subject.starts_with("branch: Created from"))
        .map(|(sha, _)| sha);
    let mut args = vec!["rev-list", "--count", "HEAD", "--not", &remote];
    args.extend(tips.lines().chain(created));
    args.push("--");
    !git(&args).is_ok_and(|n| n == "0")
}

/// The bases tried in order when origin has no `HEAD`; the help of `--base` names them (#322).
pub const BASES: [&str; 5] = [
    "origin/master",
    "origin/main",
    "origin/develop",
    "master",
    "main",
];

/// `origin/HEAD`, else the first of [`BASES`] that exists.
fn detect_base(git: &dyn Fn(&[&str]) -> Result<String>) -> Result<String> {
    if let Ok(b) = git(&["symbolic-ref", "-q", "--short", "refs/remotes/origin/HEAD"]) {
        return Ok(b);
    }
    for b in BASES {
        if git(&["rev-parse", "--verify", "-q", &format!("{b}^{{commit}}")]).is_ok() {
            return Ok(b.to_string());
        }
    }
    bail!("no base branch found: pass --base")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Lines deleted above the first line are marked on it.
        assert_eq!(
            parse("@@ -1,3 +0,0 @@\n", false).marks[&0],
            Mark::DeletedBelow
        );
        assert!(parse("", false).marks.is_empty());
    }

    #[test]
    fn review_keeps_deleted_lines_as_ghosts_and_hunk_starts() {
        let diff = "@@ -1 +1 @@\n-x\n+y\n\
                    @@ -3,0 +4,2 @@\n+a\n+b\n\
                    @@ -8,2 +9,0 @@\n-c\n-d\n\\ No newline at end of file\n";
        let d = parse(diff, true);
        use TextLine::{Deleted, File};
        assert_eq!(d.hunks, vec![Deleted(0, 0), File(3), Deleted(9, 0)]);
        assert_eq!(d.ghosts[&0], vec!["x"]);
        assert_eq!(d.ghosts[&9], vec!["c", "d"]);
        assert_eq!(d.ghosts.get(&3), None);
        // A changed line is an added one under its ghost: no `Changed` in review.
        assert_eq!(d.marks[&0], Mark::Added);
        assert_eq!(d.marks.len(), 3, "{:?}", d.marks);
        // A deletion right after a changed last line is one stop, not two.
        let d = parse("@@ -5 +5 @@\n-a\n+b\n@@ -6,2 +5,0 @@\n-c\n-d\n", true);
        assert_eq!(
            (d.hunks.clone(), d.ghosts[&5].len()),
            (vec![Deleted(4, 0)], 2)
        );
    }

    #[test]
    fn review_pairs_added_lines_with_the_ghosts_they_rewrite() {
        let diff = "@@ -1,2 +1,2 @@\n-total = price * qty\n-log.debug(\"pricing %s\", item)\n\
                    +total = price * qty * rate\n+log.debug(\"pricing %s\", item)\n\
                    @@ -10 +10,2 @@\n-total = price * qty\n\
                    +log.debug(\"pricing %s\", item)\n+total = price * qty * rate\n";
        let d = parse(diff, true);
        // The first ghost at each key is this 0-based base-file line.
        assert_eq!(d.ghost_from, BTreeMap::from([(0, 0), (9, 9)]));
        // A balanced hunk pairs in order; an unbalanced one by similarity.
        assert_eq!(
            d.pairs,
            HashMap::from([(0, (0, 0)), (1, (0, 1)), (10, (9, 0))])
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
        let _ = std::fs::remove_dir_all(&dir);
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

    /// A user's `diff.interHunkContext` joins nearby edits into one hunk with the unchanged
    /// lines between them: each edit stays its own hunk.
    #[test]
    fn nearby_edits_stay_apart_under_inter_hunk_context() {
        let dir = std::env::temp_dir().join(format!("merl-interhunk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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
        use TextLine::Deleted;
        assert_eq!(d.hunks, vec![Deleted(2, 0), Deleted(5, 0)]);
        assert_eq!(d.ghosts[&5], vec!["6"]);
    }

    /// #221: a name is the file, not a pattern: `[id].tsx` would match `d.tsx` too.
    #[test]
    fn a_name_with_glob_characters_is_that_file_alone() {
        let dir = std::env::temp_dir().join(format!("merl-glob-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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
    fn a_review_lists_the_branch_files_against_the_merge_base() {
        let dir = std::env::temp_dir().join(format!("merl-review-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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
        // A rename with an edit, and a non-ASCII name git would quote without `-z`.
        std::fs::rename(dir.join("moved"), dir.join("src/moved.rs")).unwrap();
        std::fs::write(dir.join("src/moved.rs"), format!("{ten}M10\n")).unwrap();
        std::fs::write(dir.join("файл.txt"), "ф\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "work"]);
        // Base moves on: the diff is still against the merge base, not the base tip.
        git(&["switch", "-q", "main"]);
        std::fs::write(dir.join("other"), "o\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "later"]);
        git(&["switch", "-q", "feature"]);

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
        // The rename diffs against its old name: one hunk, not a whole new file.
        let moved = &r.files[1];
        assert_eq!(moved.old.as_deref(), Some(Path::new("moved")));
        let d = diff(
            &dir,
            &dir.join(&moved.path),
            Some(&r.merge_base),
            moved.old.as_deref(),
        );
        use TextLine::{Deleted, File};
        assert_eq!(d.hunks, vec![Deleted(9, 0)]);
        assert_eq!(d.ghosts[&9], vec!["m10"]);
        let d = diff(&dir, &dir.join("src/a.rs"), Some(&r.merge_base), None);
        assert_eq!(d.hunks, vec![Deleted(1, 0), File(3)]);
        assert_eq!(d.ghosts[&1], vec!["b"]);
        assert_eq!(r.base_bytes(&dir, Path::new("gone")).unwrap(), b"x\n");
        assert!(
            Review::open(&dir, None, Some("feature")).is_err(),
            "no changes"
        );
        // Where the branch lives, for the watch: a linked worktree has its own `HEAD`.
        let real = dir.canonicalize().unwrap();
        let wt = real.with_extension("wt");
        let _ = std::fs::remove_dir_all(&wt);
        git(&["worktree", "add", "-q", "-b", "wt", wt.to_str().unwrap()]);
        let common = real.join(".git");
        assert_eq!(dirs(&dir), Some((common.clone(), common.clone())));
        let (git_dir, common_dir) = dirs(&wt).unwrap();
        assert!(git_dir.starts_with(common.join("worktrees")) && git_dir.join("HEAD").exists());
        assert_eq!(common_dir, common);
        std::fs::remove_dir_all(&wt).unwrap();
        // With a branch name the review switches to it; a dirty tree in the way is an error.
        git(&["switch", "-q", "main"]);
        let r = Review::open(&dir, Some("feature"), None).unwrap();
        assert_eq!(r.branch, "feature");
        git(&["switch", "-q", "main"]);
        std::fs::write(dir.join("src/a.rs"), "dirty\n").unwrap();
        assert!(
            Review::open(&dir, Some("feature"), None).is_err(),
            "dirty switch"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #396: `-r BRANCH` for a branch an agent's worktree has checked out reviews it there.
    #[test]
    fn a_branch_checked_out_in_another_worktree_is_reviewed_there() {
        let tmp = std::env::temp_dir().canonicalize().unwrap();
        let dir = tmp.join(format!("merl-other-wt-{}", std::process::id()));
        let wt = dir.with_extension("agent");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&wt);
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
        // Not committed yet: the agent is still at it.
        std::fs::write(wt.join("new.py"), "y = 3\n").unwrap();

        assert_eq!(worktree_of(&dir, "agent/task"), Some(wt.clone()));
        // The branch of this very worktree, or of none, is today's `-r BRANCH`.
        assert_eq!(worktree_of(&dir, "main"), None);
        assert_eq!(worktree_of(&dir, "nowhere"), None);
        let r = Review::open(&wt, None, None).unwrap();
        assert_eq!((r.branch.as_str(), r.base.as_str()), ("agent/task", "main"));
        let rows: Vec<_> = r
            .files
            .iter()
            .map(|f| (f.status, f.path.to_str().unwrap()))
            .collect();
        assert_eq!(rows, vec![('M', "a.py"), ('A', "new.py")]);
        // Nothing switched on either side.
        let head = |at: &Path| git(at, &["symbolic-ref", "--short", "HEAD"]);
        assert_eq!(
            (head(&dir), head(&wt)),
            ("main\n".into(), "agent/task\n".into())
        );

        // A worktree whose directory is gone is `prunable`: the error stays git's.
        std::fs::remove_dir_all(&wt).unwrap();
        assert_eq!(worktree_of(&dir, "agent/task"), None);
        let e = Review::open(&dir, Some("agent/task"), None).unwrap_err();
        assert!(
            format!("{e:#}").contains("switching to agent/task"),
            "{e:#}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #76: a branch whose only change is not in git yet is a review.
    #[test]
    fn untracked_files_are_rows_of_the_review() {
        let dir = std::env::temp_dir().join(format!("merl-untracked-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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
        // A name git lists first and in spaces, an empty file, text with a NUL past the 8000
        // bytes git looks at, a binary, an ignored file, and a nested repository (`sub/`).
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
            ]
        );
        assert!(r.files.iter().all(|f| f.status == 'A' && f.untracked));
        let d = r.diff(&dir, &dir.join("empty.py"), r.file(Path::new("empty.py")));
        assert_eq!(d, Diff::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #405: git lists an untracked link to a FIFO, and counting its lines opened the FIFO and
    /// waited for a writer forever, before the first frame. On a thread of its own here, so a
    /// wait fails the test instead of hanging it.
    #[cfg(unix)]
    #[test]
    fn an_untracked_link_to_a_fifo_is_a_row_that_is_not_read() {
        let dir = std::env::temp_dir().join(format!("merl-fifo-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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
            let _ = tx.send(
                r.files
                    .iter()
                    .map(|f| (f.path.clone(), f.has_hunks()))
                    .collect(),
            );
        });
        let rows: Vec<(PathBuf, bool)> =
            rx.recv_timeout(std::time::Duration::from_secs(60)).unwrap();
        assert_eq!(rows, [(PathBuf::from("pipe.link"), false)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #404: the review opens on the first file with something to read; a branch of nothing
    /// but a link to a directory opens on no file, where `Buffer::load` on the link ended the
    /// start, and a branch of binaries opens on one.
    #[cfg(unix)]
    #[test]
    fn a_review_opens_on_a_file_with_text_a_binary_or_none() {
        let dir = std::env::temp_dir().join(format!("merl-first-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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
        let first = |dir: &Path| Review::open(dir, None, None).unwrap().first_file(dir);
        assert_eq!(first(&dir), None);
        std::fs::write(dir.join("logo.png"), b"\x89PNG\0\n").unwrap();
        assert_eq!(first(&dir), Some(dir.join("logo.png")));
        std::fs::write(dir.join("z.py"), "x = 1\n").unwrap();
        assert_eq!(first(&dir), Some(dir.join("z.py")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #181: `--review feat` reads the branch and the base as `origin` has them.
    #[test]
    fn a_named_review_reads_what_was_pushed() {
        let dir = std::env::temp_dir().join(format!("merl-pushed-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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
        assert_eq!((names(&r), r.note), (vec!["one".to_string()], None));
        git("b", &["switch", "-q", "main"]);

        // Rebased and force-pushed: a copy nobody committed on takes the new history, though
        // `git clone` left no trace of the tip it was created from.
        git("a", &["reset", "-q", "--hard", "HEAD~1"]);
        commit("a", "two");
        push();
        assert_eq!(names(&open()), ["two"]);
        git("b", &["switch", "-q", "main"]);

        // Pushed since: the local branch from the last review moves on to it.
        commit("a", "three");
        push();
        let r = open();
        assert_eq!(names(&r), ["three", "two"]);
        assert_eq!(r.note, None);

        // The branch took in a base that moved on: that commit is the base's, not the branch's,
        // though this clone never fetched it.
        git("a", &["switch", "-q", "main"]);
        commit("a", "later");
        push();
        git("a", &["switch", "-q", "feat"]);
        git("a", &["merge", "-q", "--no-edit", "main"]);
        push();
        assert_eq!(names(&open()), ["three", "two"]);

        // Local changes in the way: the branch stays, and the review says it is behind.
        std::fs::write(dir.join("b/two"), "mine\n").unwrap();
        std::fs::write(dir.join("a/two"), "fixed\n").unwrap();
        git("a", &["commit", "-q", "-am", "fix"]);
        push();
        assert_eq!(open().note.as_deref(), Some("behind origin/feat"));
        git("b", &["checkout", "-q", "--", "two"]);
        assert_eq!(open().note, None);

        // The reviewer's own commit stays, ahead of what was pushed or diverged from it.
        commit("b", "mine");
        let r = open();
        assert_eq!((names(&r).contains(&"mine".into()), r.note), (true, None));
        commit("a", "four");
        push();
        let r = open();
        assert_eq!(names(&r), ["mine", "three", "two"]);
        assert_eq!(r.note.as_deref(), Some("diverged from origin/feat"));

        // Nothing to fetch from: the branch as it is, and the review says so.
        git("b", &["reset", "-q", "--hard", "origin/feat~1"]);
        std::fs::remove_dir_all(&remote).unwrap();
        let r = open();
        assert_eq!(names(&r), ["three", "two"]);
        assert_eq!(r.note.as_deref(), Some("origin/feat not fetched"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #271: `-r origin/feat` is `-r feat` that origin must have.
    #[test]
    fn a_review_of_origin_slash_branch_reads_the_pushed_branch() {
        let dir = std::env::temp_dir().join(format!("merl-origin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
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
        let open = |branch: &str| Review::open(&b, Some(branch), None);
        let error = |branch: &str| format!("{:#}", open(branch).unwrap_err());
        git("", &["init", "-q", "--bare", "-b", "main", "remote.git"]);
        git("", &["clone", "-q", "remote.git", "a"]);
        git("a", &["switch", "-q", "-c", "main"]);
        commit("a", "README");
        git("a", &["push", "-q", "origin", "main"]);
        git("", &["clone", "-q", "remote.git", "b"]);
        git("a", &["switch", "-q", "-c", "feat"]);
        commit("a", "one");
        git("a", &["push", "-q", "origin", "feat"]);

        // Fetched, and the local `feat` made from it, as `-r feat` does.
        let r = open("origin/feat").unwrap();
        assert_eq!((r.branch.as_str(), r.note), ("feat", None));
        assert_eq!(r.files[0].path, Path::new("one"));
        git("b", &["switch", "-q", "main"]);

        // Not on origin: an error, though a local branch of that name exists.
        git("b", &["branch", "mine"]);
        assert_eq!(error("origin/mine"), "no branch mine on origin");
        // Another remote's prefix is part of a local name, as before.
        assert!(error("upstream/feat").starts_with("switching to upstream/feat"));

        // A local branch literally named `origin/lit` is that branch.
        git("b", &["switch", "-q", "-c", "origin/lit"]);
        commit("b", "lit");
        git("b", &["switch", "-q", "main"]);
        assert_eq!(open("origin/lit").unwrap().branch, "origin/lit");
        git("b", &["switch", "-q", "main"]);

        // Origin unreachable: the local branch, and the review says it was not fetched.
        std::fs::remove_dir_all(dir.join("remote.git")).unwrap();
        let r = open("origin/feat").unwrap();
        assert_eq!(r.branch, "feat");
        assert_eq!(r.note.as_deref(), Some("origin/feat not fetched"));
        std::fs::remove_dir_all(&dir).unwrap();
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

    /// #243: generated as both GitHub and GitLab have it: a name both fold, a directory both
    /// fold, Go's header in the first 40 lines, or `.gitattributes`, which also keeps a lock
    /// file open.
    #[test]
    fn generated_files_are_known_by_name_attribute_and_header() {
        let dir = std::env::temp_dir().join(format!("merl-generated-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
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
            // Neither forge folds these; the attributes keep a lock file open.
            ("yarn.lock", "x\n", false),
            ("go.sum", "x\n", false),
            ("Gemfile.lock", "x\n", false),
            ("open/poetry.lock", "x\n", false),
            ("open/package-lock.json", "{}\n", false),
            ("no.txt", "n\n", false),
            ("unset.txt", "u\n", false),
            // Near misses: another name, a header past line 40, half a header, not Go.
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
        // Untracked: read the same way.
        std::fs::write(dir.join("gen/new.ts"), "n\n").unwrap();
        std::fs::write(dir.join("new.go"), header).unwrap();
        let r = Review::open(&dir, None, None).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let mut want: Vec<_> = files.iter().map(|&(n, _, g)| (n, g)).collect();
        want.extend([("gen/new.ts", true), ("new.go", true)]);
        want.sort();
        let mut got: Vec<_> = (r.files.iter())
            .map(|f| (f.path.to_str().unwrap(), f.generated))
            .collect();
        got.sort();
        assert_eq!(got, want);
    }
}
