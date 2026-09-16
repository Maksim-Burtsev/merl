//! The file tree: one snapshot of the project taken at startup.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

pub struct Node {
    /// Path relative to the project root.
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
}

impl Node {
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or(self.path.as_os_str())
            .to_string_lossy()
            .into_owned()
    }
}

/// A flat, pre-ordered list of nodes; nesting lives in `depth` alone.
#[derive(Default)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub cursor: usize,
}

/// Walks `root` once and returns the tree plus the flat list of files, both sorted the same
/// way: inside every directory, directories first, then files, case-insensitively by name.
///
/// ponytail: startup snapshot. New or deleted files need a restart; a watcher over the whole
/// tree would be a second source of truth for a read-only viewer.
pub fn build(root: &Path) -> (Tree, Vec<PathBuf>) {
    // Dotfiles are walked: `.github/`, `.env` and `.dockerignore` are part of a project.
    // `.gitignore` still prunes caches; a version-control store is never content.
    let mut entries: Vec<(PathBuf, bool)> = WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .filter_entry(|e| !matches!(e.file_name().to_str(), Some(".git" | ".hg" | ".svn")))
        .build()
        .filter_map(Result::ok)
        .filter_map(|e| {
            let rel = e.path().strip_prefix(root).ok()?.to_path_buf();
            // The root itself is the pane title, not a row.
            (!rel.as_os_str().is_empty()).then(|| (rel, e.file_type().is_some_and(|t| t.is_dir())))
        })
        .collect();
    entries.sort_by_cached_key(|(p, is_dir)| sort_key(p, *is_dir));

    let files = entries
        .iter()
        .filter(|(_, is_dir)| !is_dir)
        .map(|(p, _)| p.clone())
        .collect();
    (from_entries(entries, false), files)
}

/// A tree of just `files` (relative paths) and the directories above them, all expanded:
/// the review panel.
pub fn from_files(files: &[PathBuf]) -> Tree {
    let mut entries: Vec<(PathBuf, bool)> = files.iter().map(|p| (p.clone(), false)).collect();
    for f in files {
        for dir in f.ancestors().skip(1) {
            if !dir.as_os_str().is_empty() && !entries.iter().any(|(p, _)| p == dir) {
                entries.push((dir.to_path_buf(), true));
            }
        }
    }
    entries.sort_by_cached_key(|(p, is_dir)| sort_key(p, *is_dir));
    from_entries(entries, true)
}

fn from_entries(entries: Vec<(PathBuf, bool)>, expanded: bool) -> Tree {
    let nodes = entries
        .into_iter()
        .map(|(path, is_dir)| Node {
            depth: path.components().count() - 1,
            path,
            is_dir,
            expanded,
        })
        .collect();
    Tree { nodes, cursor: 0 }
}

/// Sorting a path component by component puts every child right after its parent, and the
/// `0`/`1` rank puts directories before files at each level.
pub(crate) fn sort_key(path: &Path, is_dir: bool) -> Vec<(u8, String)> {
    let last = path.components().count() - 1;
    path.components()
        .enumerate()
        .map(|(i, c)| {
            let rank = u8::from(i == last && !is_dir);
            (rank, c.as_os_str().to_string_lossy().to_lowercase())
        })
        .collect()
}

impl Tree {
    /// Indices of the nodes whose ancestors are all expanded, in display order.
    pub fn visible(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut hidden_below = usize::MAX;
        for (i, n) in self.nodes.iter().enumerate() {
            if n.depth > hidden_below {
                continue;
            }
            hidden_below = usize::MAX;
            out.push(i);
            if n.is_dir && !n.expanded {
                hidden_below = n.depth;
            }
        }
        out
    }

    pub fn selected(&self) -> Option<&Node> {
        self.nodes.get(self.cursor)
    }

    fn move_by(&mut self, delta: isize) {
        let vis = self.visible();
        let Some(at) = vis.iter().position(|&i| i == self.cursor) else {
            self.cursor = vis.first().copied().unwrap_or(0);
            return;
        };
        let next = at.saturating_add_signed(delta).min(vis.len() - 1);
        self.cursor = vis[next];
    }

    pub fn up(&mut self) {
        self.move_by(-1);
    }

    pub fn down(&mut self) {
        self.move_by(1);
    }

    /// Expands or collapses the selected directory; a file is left alone.
    pub fn toggle(&mut self) {
        if let Some(n) = self.nodes.get_mut(self.cursor)
            && n.is_dir
        {
            n.expanded = !n.expanded;
        }
    }

    pub fn expand(&mut self) {
        if let Some(n) = self.nodes.get_mut(self.cursor)
            && n.is_dir
        {
            n.expanded = true;
        }
    }

    /// Collapses an expanded directory; otherwise jumps to the parent directory.
    pub fn collapse(&mut self) {
        match self.nodes.get_mut(self.cursor) {
            Some(n) if n.is_dir && n.expanded => n.expanded = false,
            Some(n) if n.depth > 0 => {
                let depth = n.depth;
                if let Some(p) = self.nodes[..self.cursor]
                    .iter()
                    .rposition(|n| n.depth == depth - 1)
                {
                    self.cursor = p;
                }
            }
            _ => {}
        }
    }

    /// Expands every ancestor of `rel` and puts the cursor on it. Unknown paths are ignored.
    pub fn reveal(&mut self, rel: &Path) {
        let Some(at) = self.nodes.iter().position(|n| n.path == rel) else {
            return;
        };
        for n in &mut self.nodes[..at] {
            if n.is_dir && rel.starts_with(&n.path) {
                n.expanded = true;
            }
        }
        self.cursor = at;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tag: &str) -> Tree {
        let dir = std::env::temp_dir().join(format!("merl-tree-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/deep")).unwrap();
        for f in ["Zed.toml", "aaa.rs", "src/app.rs", "src/deep/x.rs"] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let (tree, files) = build(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        // Directories first, then files, both case-insensitive.
        assert_eq!(
            tree.nodes.iter().map(Node::name).collect::<Vec<_>>(),
            ["src", "deep", "x.rs", "app.rs", "aaa.rs", "Zed.toml"]
        );
        assert_eq!(
            files,
            [
                PathBuf::from("src/deep/x.rs"),
                PathBuf::from("src/app.rs"),
                PathBuf::from("aaa.rs"),
                PathBuf::from("Zed.toml"),
            ]
        );
        tree
    }

    #[test]
    fn from_files_adds_the_directories_above_them_expanded() {
        let t = from_files(&["src/b/y.rs".into(), "README".into(), "src/a.rs".into()]);
        let rows: Vec<_> = t
            .visible()
            .iter()
            .map(|&i| (t.nodes[i].depth, t.nodes[i].path.to_str().unwrap()))
            .collect();
        assert_eq!(
            rows,
            vec![
                (0, "src"),
                (1, "src/b"),
                (2, "src/b/y.rs"),
                (1, "src/a.rs"),
                (0, "README")
            ]
        );
    }

    #[test]
    fn collapsed_by_default_and_toggles() {
        let mut t = fixture("toggle");
        assert_eq!(t.visible().len(), 3, "only the root level shows");
        t.toggle(); // src
        assert_eq!(
            t.visible()
                .iter()
                .map(|&i| t.nodes[i].name())
                .collect::<Vec<_>>(),
            ["src", "deep", "app.rs", "aaa.rs", "Zed.toml"]
        );
        t.down();
        assert_eq!(t.selected().unwrap().name(), "deep");
        t.collapse(); // already collapsed -> go to the parent
        assert_eq!(t.selected().unwrap().name(), "src");
        t.collapse();
        assert_eq!(t.visible().len(), 3);
    }

    #[test]
    fn reveal_expands_ancestors() {
        let mut t = fixture("reveal");
        t.reveal(Path::new("src/deep/x.rs"));
        assert_eq!(t.selected().unwrap().name(), "x.rs");
        assert_eq!(t.visible().len(), 6);
        // Down from the deepest node stops at the last visible row.
        for _ in 0..10 {
            t.down();
        }
        assert_eq!(t.selected().unwrap().name(), "Zed.toml");
    }

    #[test]
    fn dotfiles_are_walked_but_git_and_ignored_files_are_not() {
        let dir = std::env::temp_dir().join(format!("merl-tree-{}-dot", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for d in [".git", ".hg", ".svn", ".github/workflows", ".cache"] {
            std::fs::create_dir_all(dir.join(d)).unwrap();
        }
        for (f, text) in [
            (".git/HEAD", "ref: refs/heads/master"),
            (".hg/requires", "store"),
            (".svn/wc.db", "x"),
            (".github/workflows/ci.yml", "on: push"),
            (".env", "PORT=1"),
            (".gitignore", ".cache/"),
            (".cache/blob", "x"),
        ] {
            std::fs::write(dir.join(f), text).unwrap();
        }
        let (_, files) = build(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            files,
            [
                PathBuf::from(".github/workflows/ci.yml"),
                PathBuf::from(".env"),
                PathBuf::from(".gitignore"),
            ]
        );
    }
}
