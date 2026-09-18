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

/// What one file's diff looks like from the editor: marks per 0-based line, and, in review
/// mode, the deleted text as ghost lines keyed by the line they sit above (`lines.len()` for
/// the end of the file) plus the first line of every hunk, for `c` / `C`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    pub marks: HashMap<usize, Mark>,
    pub ghosts: BTreeMap<usize, Vec<String>>,
    pub hunks: Vec<usize>,
}

impl Diff {
    pub fn ghost_n(&self, line: usize) -> usize {
        self.ghosts.get(&line).map_or(0, Vec::len)
    }
}

/// The diff of `path` in the working tree against the index, or against `base` when given
/// (review mode: ghosts and hunks are only kept then). `old` is the name the file had at the
/// base when the branch renamed it: with both names in the pathspec git pairs them.
pub fn diff(root: &Path, path: &Path, base: Option<&str>, old: Option<&Path>) -> Diff {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .args(["diff", "-U0", "-M", "--no-color", "--no-ext-diff"]);
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
    while let Some(line) = lines.next() {
        if !line.starts_with("@@ ") {
            continue;
        }
        let Some((old, new)) = line[3..].split_once(" +") else {
            continue;
        };
        let (Some(old_n), Some((new_start, new_n))) = (
            count(old.trim_start_matches('-')),
            range(new.split(' ').next().unwrap_or("")),
        ) else {
            continue;
        };
        // 0-based line the hunk's new text starts on; a pure deletion sits above `new_start + 1`.
        let at = if new_n == 0 { new_start } else { new_start - 1 };
        if review {
            let mut deleted = Vec::new();
            while let Some(l) =
                lines.next_if(|l| l.starts_with('-') || l.starts_with('+') || l.starts_with('\\'))
            {
                if let Some(text) = l.strip_prefix('-') {
                    deleted.push(text.to_string());
                }
            }
            if !deleted.is_empty() {
                out.ghosts.entry(at).or_default().extend(deleted);
            }
            // A deletion at the end of the file is drawn under the last line; `c` stands on
            // that line for it.
            let stop = if new_n == 0 && at > 0 { at - 1 } else { at };
            if out.hunks.last() != Some(&stop) {
                out.hunks.push(stop);
            }
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

fn count(s: &str) -> Option<usize> {
    range(s).map(|(_, n)| n)
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
}

impl Review {
    /// Checks out `branch` when given (after a fetch), finds the base and lists the files.
    pub fn open(root: &Path, branch: Option<&str>, base: Option<&str>) -> Result<Self> {
        let git = |args: &[&str]| git(root, args);
        if let Some(b) = branch {
            let _ = git(&["fetch", "-q", "origin", b]);
            git(&["switch", "-q", b]).with_context(|| format!("switching to {b}"))?;
        }
        let base = match base {
            Some(b) => b.to_string(),
            None => detect_base(&git)?,
        };
        let r = Self::list(root, base)?;
        if r.files.is_empty() {
            bail!("{} has no changes against {}", r.branch, r.base);
        }
        Ok(r)
    }

    /// The same review as the branch and the working tree are now: after a commit, an edit, a
    /// new file. Nothing is fetched or switched, and an empty list is an answer.
    pub fn refresh(&self, root: &Path) -> Result<Self> {
        Self::list(root, self.base.clone())
    }

    fn list(root: &Path, base: String) -> Result<Self> {
        let git = |args: &[&str]| git(root, args);
        let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        let merge_base = git(&["merge-base", &base, "HEAD"])
            .with_context(|| format!("no merge base between {base} and HEAD"))?;
        // `-z`: NUL-separated and unquoted, so a non-ASCII name is the name on disk.
        let mut files = parse_name_status(&git(&["diff", "--name-status", "-z", &merge_base])?);
        for (path, counts) in parse_numstat(&git(&["diff", "--numstat", "-z", &merge_base])?) {
            if let Some(f) = files.iter_mut().find(|f| f.path == path) {
                f.binary = counts.is_none();
                (f.added, f.deleted) = counts.unwrap_or((0, 0));
            }
        }
        // `git diff` does not list untracked files, and a new module is the first thing to
        // review. A nested repository is listed as `dir/`: not a file to read.
        let others = git(&["ls-files", "--others", "--exclude-standard", "-z"])?;
        for path in others
            .split('\0')
            .filter(|p| !p.is_empty() && !p.ends_with('/'))
        {
            files.push(untracked(root, Path::new(path)));
        }
        // The panel's order, so `c` walks the files top to bottom.
        files.sort_by_cached_key(|f| crate::tree::sort_key(&f.path, false));
        Ok(Self {
            branch,
            base,
            merge_base,
            files,
        })
    }

    /// The marks, ghosts and hunks of one file of the working tree against the merge base;
    /// `file` is its row, when it has one. An untracked file is one hunk of added lines.
    pub fn diff(&self, root: &Path, path: &Path, file: Option<&ReviewFile>) -> Diff {
        match file {
            Some(f) if f.untracked => {
                let n = std::fs::read(path).map_or(0, |b| line_count(&b));
                Diff {
                    marks: (0..n).map(|l| (l, Mark::Added)).collect(),
                    hunks: Vec::from_iter((n > 0).then_some(0)),
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
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The git directory of the worktree at `root` (where `HEAD` is) and the repository's common
/// one (where the refs are), canonical. They are the same `.git` outside a linked worktree.
pub fn dirs(root: &Path) -> Option<(PathBuf, PathBuf)> {
    let out = git(root, &["rev-parse", "--git-dir", "--git-common-dir"]).ok()?;
    let mut dirs = out.lines().map(|d| root.join(d).canonicalize().ok());
    Some((dirs.next()??, dirs.next()??))
}

/// The row of an untracked file: `A`, every line added. Binary is what git calls binary, a NUL
/// in the first 8000 bytes.
///
/// ponytail: every untracked file is read on every refresh to count its lines. A branch with
/// thousands of them wants the counts cached by mtime.
fn untracked(root: &Path, path: &Path) -> ReviewFile {
    let bytes = std::fs::read(root.join(path)).unwrap_or_default();
    let binary = bytes.iter().take(8000).any(|&b| b == 0);
    ReviewFile {
        path: path.to_path_buf(),
        status: 'A',
        old: None,
        added: if binary { 0 } else { line_count(&bytes) },
        deleted: 0,
        binary,
        untracked: true,
    }
}

/// Lines as `git diff --numstat` counts them: a last line without a newline is a line.
fn line_count(bytes: &[u8]) -> usize {
    bytes.split(|&b| b == b'\n').count() - usize::from(bytes.is_empty() || bytes.ends_with(b"\n"))
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

/// `origin/HEAD`, else the first of `origin/master`, `origin/main`, `origin/develop` that
/// exists, else the local `master` / `main`.
fn detect_base(git: &dyn Fn(&[&str]) -> Result<String>) -> Result<String> {
    if let Ok(b) = git(&["symbolic-ref", "-q", "--short", "refs/remotes/origin/HEAD"]) {
        return Ok(b);
    }
    for b in [
        "origin/master",
        "origin/main",
        "origin/develop",
        "master",
        "main",
    ] {
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
        assert_eq!(d.hunks, vec![0, 3, 8]);
        assert_eq!(d.ghosts[&0], vec!["x"]);
        assert_eq!(d.ghosts[&9], vec!["c", "d"]);
        assert_eq!(d.ghost_n(3), 0);
        // A changed line is an added one under its ghost: no `Changed` in review.
        assert_eq!(d.marks[&0], Mark::Added);
        assert_eq!(d.marks.len(), 3, "{:?}", d.marks);
        // A deletion right after a changed last line is one stop, not two.
        let d = parse("@@ -5 +5 @@\n-a\n+b\n@@ -6,2 +5,0 @@\n-c\n-d\n", true);
        assert_eq!((d.hunks.clone(), d.ghost_n(5)), (vec![4], 2));
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
        assert_eq!(d.hunks, vec![9]);
        assert_eq!(d.ghosts[&9], vec!["m10"]);
        let d = diff(&dir, &dir.join("src/a.rs"), Some(&r.merge_base), None);
        assert_eq!(d.hunks, vec![1, 3]);
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
}
