//! The file tree: the project as the last walk found it.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

pub struct Node {
    /// Path relative to the project root.
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
    /// Left out by `.gitignore` and the other ignore files: a dim row. The walk does not go into
    /// an ignored directory; it is read from disk one level at a time, when it is expanded.
    pub ignored: bool,
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
    /// Expanded directories the last walk did not list. A walk can land in the middle of a
    /// `git stash` and `pop`: what comes back comes back expanded.
    away: HashSet<PathBuf>,
    /// Where the ignored directories are read from; empty in the review panel, which has none.
    root: PathBuf,
}

/// Walks `root` once and returns the tree plus the flat list of files, both sorted the same
/// way: inside every directory, directories first, then files, case-insensitively by name.
/// Done at startup and again, off the UI thread, whenever the project changes on disk.
/// `shallow` lists only the files right in `root`: its directories would open onto nothing.
/// The list leaves out what is ignored; the tree has it as dim rows, an ignored directory
/// unread.
pub fn build(root: &Path, shallow: bool) -> (Tree, Vec<PathBuf>) {
    // Dotfiles are walked: `.github/`, `.env` and `.dockerignore` are part of a project.
    // `.gitignore` still prunes caches; a version-control store is never content.
    let walked: Vec<(PathBuf, bool)> = WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .max_depth(shallow.then_some(1))
        .filter_entry(|e| !is_store(e.file_name()))
        .build()
        .filter_map(Result::ok)
        .filter_map(|e| {
            let rel = e.path().strip_prefix(root).ok()?.to_path_buf();
            let is_dir = e.file_type().is_some_and(|t| t.is_dir());
            // The root itself is the pane title, not a row.
            (!rel.as_os_str().is_empty() && !(shallow && is_dir)).then_some((rel, is_dir))
        })
        .collect();
    // Whatever else the walked directories hold is ignored.
    // ponytail: every walked directory is read a second time for it, one after another: on
    // gitea (7k rows, 1.4k directories) the walk takes 53 ms instead of 33. Threads won 8 ms
    // back; `build_parallel` for the walk itself if a project shows the difference.
    let listed: HashSet<&Path> = walked.iter().map(|(p, _)| p.as_path()).collect();
    let dirs = walked.iter().filter(|(_, is_dir)| *is_dir);
    let ignored: Vec<(PathBuf, bool)> = std::iter::once(Path::new(""))
        .chain(dirs.map(|(p, _)| p.as_path()))
        .flat_map(|dir| read_level(root, dir))
        .filter(|(p, is_dir)| !listed.contains(p.as_path()) && !(shallow && *is_dir))
        .collect();
    let mut nodes: Vec<Node> = walked
        .into_iter()
        .map(|(p, is_dir)| node(p, is_dir, false))
        .chain(ignored.into_iter().map(|(p, is_dir)| node(p, is_dir, true)))
        .collect();
    nodes.sort_by_cached_key(|n| sort_key(&n.path, n.is_dir));

    let files = nodes
        .iter()
        .filter(|n| !n.is_dir && !n.ignored)
        .map(|n| n.path.clone())
        .collect();
    let tree = Tree {
        nodes,
        root: root.to_path_buf(),
        ..Tree::default()
    };
    (tree, files)
}

fn is_store(name: &OsStr) -> bool {
    matches!(name.to_str(), Some(".git" | ".hg" | ".svn"))
}

/// The entries of `dir` (relative to `root`) and whether each is a directory, unsorted.
fn read_level(root: &Path, dir: &Path) -> Vec<(PathBuf, bool)> {
    let Ok(read) = std::fs::read_dir(root.join(dir)) else {
        return Vec::new();
    };
    read.flatten()
        .filter(|e| !is_store(&e.file_name()))
        .map(|e| {
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            (dir.join(e.file_name()), is_dir)
        })
        .collect()
}

fn node(path: PathBuf, is_dir: bool, ignored: bool) -> Node {
    Node {
        depth: path.components().count() - 1,
        path,
        is_dir,
        expanded: false,
        ignored,
    }
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
    let nodes = entries
        .into_iter()
        .map(|(p, is_dir)| Node {
            expanded: true,
            ..node(p, is_dir, false)
        })
        .collect();
    Tree {
        nodes,
        ..Tree::default()
    }
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
    /// Takes the rows of a fresh walk and keeps what the user set, both found by path: expanded
    /// directories stay expanded, also across a walk that missed them, and the cursor stays on
    /// its entry. When that entry is gone the cursor takes the row that followed it, or the
    /// nearest one above. A directory that was not listed before comes as `fresh` has it:
    /// closed in the project tree, open in the review panel.
    pub fn refresh(&mut self, mut fresh: Tree) {
        let mut expanded = std::mem::take(&mut self.away);
        let open = self.nodes.iter().filter(|n| n.expanded);
        expanded.extend(open.map(|n| n.path.clone()));
        let known: HashSet<&Path> = self.nodes.iter().map(|n| n.path.as_path()).collect();
        for n in &mut fresh.nodes {
            n.expanded = expanded.remove(&n.path) || n.expanded && !known.contains(&*n.path);
        }
        fresh.away = expanded;
        // The walk leaves every ignored directory unread; an expanded one is read again.
        fresh.read_ignored();
        let index: HashMap<&Path, usize> = fresh
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.path.as_path(), i))
            .collect();
        let vis = self.visible();
        let at = vis.iter().position(|&i| i == self.cursor).unwrap_or(0);
        let (above, below) = vis.split_at(at.min(vis.len()));
        let cursor = below
            .iter()
            .chain(above.iter().rev())
            .find_map(|&i| index.get(self.nodes[i].path.as_path()).copied())
            .unwrap_or(0);
        fresh.cursor = cursor;
        *self = fresh;
    }

    /// Reads the expanded ignored directories that have no rows below them yet, one level each.
    /// A directory expanded before is expanded again when its parent is read.
    fn read_ignored(&mut self) {
        let mut i = 0;
        while let Some(n) = self.nodes.get(i) {
            let unread = self.nodes.get(i + 1).is_none_or(|c| c.depth <= n.depth);
            if n.ignored && n.is_dir && n.expanded && unread {
                let mut level: Vec<Node> = read_level(&self.root, &n.path)
                    .into_iter()
                    .map(|(p, is_dir)| Node {
                        expanded: self.away.remove(&p),
                        ..node(p, is_dir, true)
                    })
                    .collect();
                level.sort_by_cached_key(|c| sort_key(&c.path, c.is_dir));
                self.nodes.splice(i + 1..i + 1, level);
            }
            i += 1;
        }
    }

    /// The directories the walk went into, relative to the root: not the ignored ones.
    pub fn dirs(&self) -> HashSet<PathBuf> {
        let dirs = self.nodes.iter().filter(|n| n.is_dir && !n.ignored);
        dirs.map(|n| n.path.clone()).collect()
    }

    /// The ignored files right in the directories the walk went into: `.env`, `.envrc`,
    /// `config/local.yml`. Not what an expanded ignored directory holds.
    pub fn ignored_files(&self) -> Vec<PathBuf> {
        let dirs = self.dirs();
        let walked = |p: &Path| p.as_os_str().is_empty() || dirs.contains(p);
        let files = self.nodes.iter().filter(|n| n.ignored && !n.is_dir);
        let files = files.filter(|n| n.path.parent().is_some_and(walked));
        files.map(|n| n.path.clone()).collect()
    }

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
        self.read_ignored();
    }

    pub fn expand(&mut self) {
        if let Some(n) = self.nodes.get_mut(self.cursor)
            && n.is_dir
        {
            n.expanded = true;
        }
        self.read_ignored();
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
        let (tree, files) = build(&dir, false);
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

    /// A project on disk that the test changes between two walks.
    fn project(tag: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("merl-live-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for f in files {
            std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        dir
    }

    fn rows(t: &Tree) -> Vec<String> {
        let vis = t.visible();
        vis.iter()
            .map(|&i| t.nodes[i].path.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn refresh_keeps_the_cursor_entry_and_the_expanded_directories() {
        let dir = project(
            "keep",
            &["api/a.py", "services/deep/x.py", "services/user.py", "z.py"],
        );
        let (mut t, _) = build(&dir, false);
        t.reveal(Path::new("services/user.py"));
        assert_eq!(
            rows(&t),
            [
                "api",
                "services",
                "services/deep",
                "services/user.py",
                "z.py"
            ]
        );

        // Rows appear above the cursor, in their sorted place; a deleted one leaves.
        for f in ["aaa/new.py", "services/billing.py", "services/deep/y.py"] {
            std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        std::fs::remove_file(dir.join("z.py")).unwrap();
        let (fresh, files) = build(&dir, false);
        t.refresh(fresh);
        assert_eq!(t.selected().unwrap().path, Path::new("services/user.py"));
        assert_eq!(
            rows(&t),
            [
                "aaa",
                "api",
                "services",
                "services/deep",
                "services/billing.py",
                "services/user.py"
            ],
            "`services` stays expanded, `api` and `services/deep` collapsed, `aaa` arrives collapsed"
        );
        assert!(files.contains(&"services/deep/y.py".into()) && !files.contains(&"z.py".into()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refresh_moves_the_cursor_off_a_deleted_entry_to_its_neighbour() {
        let dir = project("gone", &["a.py", "b.py", "c.py", "lib/x.py"]);
        let (mut t, _) = build(&dir, false);
        for (delete, from, to) in [
            ("b.py", "b.py", "c.py"),    // the row that followed
            ("c.py", "c.py", "a.py"),    // the last row: the one above
            ("lib", "lib/x.py", "a.py"), // a whole directory under the cursor
        ] {
            t.reveal(Path::new(from));
            let _ = std::fs::remove_file(dir.join(delete));
            let _ = std::fs::remove_dir_all(dir.join(delete));
            t.refresh(build(&dir, false).0);
            assert_eq!(t.selected().unwrap().path, Path::new(to), "{delete}");
        }
        std::fs::remove_file(dir.join("a.py")).unwrap();
        t.refresh(build(&dir, false).0);
        assert!(t.selected().is_none() && t.nodes.is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_directory_that_one_walk_missed_comes_back_expanded() {
        let dir = project("away", &["gen/deep/x.py", "gen/y.py", "z.py"]);
        let (mut t, _) = build(&dir, false);
        t.reveal(Path::new("gen/deep/x.py"));
        let before = rows(&t);
        // `git stash -u`, a walk, `git stash pop`, a walk.
        std::fs::rename(dir.join("gen"), dir.with_extension("aside")).unwrap();
        t.refresh(build(&dir, false).0);
        assert_eq!(rows(&t), ["z.py"]);
        std::fs::rename(dir.with_extension("aside"), dir.join("gen")).unwrap();
        t.refresh(build(&dir, false).0);
        assert_eq!(rows(&t), before);
        // Collapsed by hand after that, it stays collapsed.
        t.reveal(Path::new("gen"));
        t.collapse();
        t.refresh(build(&dir, false).0);
        assert_eq!(rows(&t), ["gen", "z.py"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refresh_respects_gitignore_and_picks_up_a_changed_one() {
        let dir = project(
            "ignore",
            &[".gitignore", "a.py", "node_modules/pkg/index.js"],
        );
        std::fs::write(dir.join(".gitignore"), "node_modules/\n").unwrap();
        let (mut t, files) = build(&dir, false);
        assert_eq!(files, [PathBuf::from(".gitignore"), "a.py".into()]);
        assert!(t.dirs().is_empty(), "an ignored directory is not watched");

        std::fs::create_dir_all(dir.join("target/debug")).unwrap();
        std::fs::write(dir.join("target/debug/merl"), b"x").unwrap();
        std::fs::write(dir.join(".gitignore"), "target/\n").unwrap();
        let (fresh, files) = build(&dir, false);
        t.refresh(fresh);
        assert_eq!(
            files,
            [
                PathBuf::from("node_modules/pkg/index.js"),
                ".gitignore".into(),
                "a.py".into()
            ]
        );
        assert_eq!(t.dirs().len(), 2, "node_modules and node_modules/pkg");
        std::fs::remove_dir_all(&dir).unwrap();
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
        let (t, files) = build(&dir, false);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            files,
            [
                PathBuf::from(".github/workflows/ci.yml"),
                PathBuf::from(".env"),
                PathBuf::from(".gitignore"),
            ]
        );
        // The ignored `.cache/` is a row, and nothing below it; the stores are not.
        let row = |n: &Node| (n.path.to_str().unwrap().to_string(), n.ignored);
        assert_eq!(
            t.nodes.iter().map(row).collect::<Vec<_>>(),
            [
                (".cache".into(), true),
                (".github".into(), false),
                (".github/workflows".into(), false),
                (".github/workflows/ci.yml".into(), false),
                (".env".into(), false),
                (".gitignore".into(), false),
            ]
        );
    }

    /// #157: what `.gitignore` leaves out is in the tree, dim, and an ignored directory is read
    /// one level at a time, when it is expanded, and again after every walk while it is open.
    #[test]
    fn an_ignored_directory_is_read_when_expanded_and_stays_read_across_walks() {
        let dir = project(
            "ignored",
            &[
                ".gitignore",
                ".env",
                "a.py",
                "src/local.yml",
                "node_modules/b.js",
                "node_modules/pkg/index.js",
                "node_modules/pkg/lib/x.js",
            ],
        );
        std::fs::write(dir.join(".gitignore"), "node_modules/\n.env\nlocal.yml\n").unwrap();
        let (mut t, files) = build(&dir, false);
        assert_eq!(files, [PathBuf::from(".gitignore"), "a.py".into()]);
        assert_eq!(
            rows(&t),
            ["node_modules", "src", ".env", ".gitignore", "a.py"]
        );
        assert_eq!(t.dirs(), HashSet::from([PathBuf::from("src")]));
        assert_eq!(
            t.ignored_files(),
            [PathBuf::from("src/local.yml"), ".env".into()]
        );

        t.reveal(Path::new("node_modules"));
        t.expand();
        t.down();
        t.toggle();
        assert_eq!(t.selected().unwrap().path, Path::new("node_modules/pkg"));
        assert_eq!(
            rows(&t)[..5],
            [
                "node_modules",
                "node_modules/pkg",
                "node_modules/pkg/lib",
                "node_modules/pkg/index.js",
                "node_modules/b.js",
            ]
        );
        assert!(
            t.nodes
                .iter()
                .filter(|n| n.path.starts_with("node_modules"))
                .all(|n| n.ignored)
        );
        t.down();
        t.expand();
        t.down();
        assert_eq!(
            t.selected().unwrap().path,
            Path::new("node_modules/pkg/lib/x.js")
        );

        // A walk does not go into `node_modules/`; what was open is read again, and the
        // cursor keeps its row. What an ignored directory holds is not for `o`.
        std::fs::write(dir.join("c.py"), b"x").unwrap();
        let before = rows(&t);
        t.refresh(build(&dir, false).0);
        assert_eq!(
            t.selected().unwrap().path,
            Path::new("node_modules/pkg/lib/x.js")
        );
        assert_eq!(rows(&t)[..before.len()], before[..]);
        assert_eq!(rows(&t).last().unwrap(), "c.py");
        assert_eq!(
            t.ignored_files(),
            [PathBuf::from("src/local.yml"), ".env".into()]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
