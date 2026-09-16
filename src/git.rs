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
/// (review mode: ghosts and hunks are only kept then).
pub fn diff(root: &Path, path: &Path, base: Option<&str>) -> Diff {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .args(["diff", "-U0", "--no-color", "--no-ext-diff"]);
    if let Some(base) = base {
        cmd.arg(base);
    }
    let out = cmd.arg("--").arg(path).output();
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
            out.hunks.push(at);
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
    pub added: usize,
    pub deleted: usize,
}

/// `merl --review`: the checked-out branch against its base.
#[derive(Debug, Clone)]
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
        let git = |args: &[&str]| -> Result<String> {
            let out = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()?;
            if !out.status.success() {
                bail!(
                    "git {}: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        };
        if let Some(b) = branch {
            let _ = git(&["fetch", "-q", "origin", b]);
            git(&["switch", "-q", b]).with_context(|| format!("switching to {b}"))?;
        }
        let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        let base = match base {
            Some(b) => b.to_string(),
            None => detect_base(&git)?,
        };
        let merge_base = git(&["merge-base", &base, "HEAD"])
            .with_context(|| format!("no merge base between {base} and HEAD"))?;
        let mut files: Vec<ReviewFile> = git(&["diff", "--name-status", &merge_base])?
            .lines()
            .filter_map(|l| {
                let (status, path) = l.split_once('\t')?;
                // Renames list `old\tnew`; the new name is the one on disk.
                let path = path.rsplit('\t').next()?;
                Some(ReviewFile {
                    path: PathBuf::from(path),
                    status: status.chars().next()?,
                    added: 0,
                    deleted: 0,
                })
            })
            .collect();
        for l in git(&["diff", "--numstat", &merge_base])?.lines() {
            let mut it = l.split('\t');
            let (Some(a), Some(d), Some(p)) = (it.next(), it.next(), it.next_back()) else {
                continue;
            };
            if let Some(f) = files.iter_mut().find(|f| f.path == Path::new(p)) {
                f.added = a.parse().unwrap_or(0);
                f.deleted = d.parse().unwrap_or(0);
            }
        }
        if files.is_empty() {
            bail!("{branch} has no changes against {base}");
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
        assert_eq!(d.hunks, vec![0, 3, 9]);
        assert_eq!(d.ghosts[&0], vec!["x"]);
        assert_eq!(d.ghosts[&9], vec!["c", "d"]);
        assert_eq!(d.ghost_n(3), 0);
        // A changed line is an added one under its ghost: no `Changed` in review.
        assert_eq!(d.marks[&0], Mark::Added);
        assert_eq!(d.marks.len(), 3, "{:?}", d.marks);
    }

    #[test]
    fn outside_git_there_are_no_marks() {
        let dir = std::env::temp_dir().join(format!("merl-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("f"), "x\n").unwrap();
        assert!(diff(&dir, &dir.join("f"), None).marks.is_empty());
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
        let m = diff(&dir, &dir.join("f"), None).marks;
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
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "base"]);
        git(&["switch", "-q", "-c", "feature"]);
        std::fs::write(dir.join("src/a.rs"), "a\nB\nc\nd\n").unwrap();
        std::fs::write(dir.join("new"), "n\n").unwrap();
        std::fs::remove_file(dir.join("gone")).unwrap();
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
                ('D', "gone", 0, 1),
                ('A', "new", 1, 0)
            ]
        );
        let d = diff(&dir, &dir.join("src/a.rs"), Some(&r.merge_base));
        assert_eq!(d.hunks, vec![1, 3]);
        assert_eq!(d.ghosts[&1], vec!["b"]);
        assert_eq!(r.base_bytes(&dir, Path::new("gone")).unwrap(), b"x\n");
        assert!(
            Review::open(&dir, None, Some("feature")).is_err(),
            "no changes"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
