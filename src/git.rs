//! Gutter marks: which lines differ from the git index, as VS Code's editor gutter shows them.
//! No diff of our own: `git diff -U0` is run after every save and reload, and only its hunk
//! headers are read.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Added,
    Changed,
    /// Lines were removed right below this one.
    DeletedBelow,
}

/// Marks for `path`, keyed by 0-based line. Empty for untracked files and outside git.
pub fn marks(root: &Path, path: &Path) -> HashMap<usize, Mark> {
    let out = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["diff", "-U0", "--no-color", "--no-ext-diff", "--"])
        .arg(path)
        .output();
    match out {
        Ok(o) if o.status.success() => parse(&String::from_utf8_lossy(&o.stdout)),
        _ => HashMap::new(),
    }
}

/// Reads `@@ -a,b +c,d @@` headers; `b` and `d` default to 1 when absent.
fn parse(diff: &str) -> HashMap<usize, Mark> {
    let mut marks = HashMap::new();
    for line in diff.lines().filter(|l| l.starts_with("@@ ")) {
        let Some((old, new)) = line[3..].split_once(" +") else {
            continue;
        };
        let (Some(old_n), Some((new_start, new_n))) = (
            count(old.trim_start_matches('-')),
            range(new.split(' ').next().unwrap_or("")),
        ) else {
            continue;
        };
        if new_n == 0 {
            // A pure deletion after 1-based line `new_start`; after "line 0" means above line 1.
            marks.insert(new_start.saturating_sub(1), Mark::DeletedBelow);
            continue;
        }
        let mark = if old_n == 0 {
            Mark::Added
        } else {
            Mark::Changed
        };
        for l in new_start - 1..new_start - 1 + new_n {
            marks.insert(l, mark);
        }
    }
    marks
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunk_headers_become_marks() {
        let diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n\
                    @@ -1 +1 @@\n-x\n+y\n\
                    @@ -3,0 +4,2 @@\n+a\n+b\n\
                    @@ -8,2 +9,0 @@\n-c\n-d\n";
        let m = parse(diff);
        assert_eq!(m[&0], Mark::Changed);
        assert_eq!((m[&3], m[&4]), (Mark::Added, Mark::Added));
        assert_eq!(m[&8], Mark::DeletedBelow);
        assert_eq!(m.len(), 4, "{m:?}");
        // Lines deleted above the first line are marked on it.
        assert_eq!(parse("@@ -1,3 +0,0 @@\n")[&0], Mark::DeletedBelow);
        assert!(parse("").is_empty());
    }

    #[test]
    fn outside_git_there_are_no_marks() {
        let dir = std::env::temp_dir().join(format!("merl-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("f"), "x\n").unwrap();
        assert!(marks(&dir, &dir.join("f")).is_empty());
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
        let m = marks(&dir, &dir.join("f"));
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(m, HashMap::from([(1, Mark::Changed), (3, Mark::Added)]));
    }
}
