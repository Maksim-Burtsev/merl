//! The file tree: the project as the last walk found it.

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::FileType;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

pub struct Node {
    /// Path relative to the project root.
    pub path: PathBuf,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
    /// Left out by `.gitignore` and the other ignore files, or a link to a directory, which the
    /// walk does not follow either: a dim row. The walk does not go into
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
    folded: HashSet<usize>,
    lift: Vec<usize>,
}

/// Walks `root` once and returns the tree plus the flat list of files, both sorted the same
/// way: inside every directory, directories first, then files, case-insensitively by name.
/// Done at startup and again, off the UI thread, whenever the project changes on disk.
/// `shallow` lists only the files right in `root`: its directories would open onto nothing.
/// The list leaves out what is ignored; the tree has it as dim rows, an ignored directory
/// unread.
pub fn build(root: &Path, shallow: bool) -> (Tree, Vec<PathBuf>) {
    build_ordered(root, shallow, &Order::default())
}

pub fn build_ordered(root: &Path, shallow: bool, order: &Order) -> (Tree, Vec<PathBuf>) {
    // Dotfiles are walked: `.github/`, `.env` and `.dockerignore` are part of a project.
    // `.gitignore` still prunes caches; a version-control store is never content.
    // A link to a directory is not followed (#404): the level it is in lists it, a directory
    // read when it is expanded, so a link to `..` or `/` loops nothing and pulls nothing in.
    let walked: Vec<(PathBuf, bool)> = WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .max_depth(shallow.then_some(1))
        .filter_entry(|e| !is_store(e.file_name()) && !(e.path_is_symlink() && e.path().is_dir()))
        .build()
        .filter_map(Result::ok)
        .filter(|e| listable(e.path(), e.file_type()))
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
    nodes.sort_by_cached_key(|n| sort_key(&n.path, n.is_dir, &Order::default()));

    let files = nodes
        .iter()
        .filter(|n| !n.is_dir && !n.ignored)
        .map(|n| n.path.clone())
        .collect();
    if !order.0.is_empty() {
        nodes.sort_by_cached_key(|n| sort_key(&n.path, n.is_dir, order));
    }
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

/// Is it a row: a directory, a regular file, a link to one, or a link that leads nowhere, a row
/// as `ls` shows it, which fails to open at once. A FIFO, a socket or a device is not code, and
/// opening one blocks until something writes to it (#405). `kind` is the entry's own type.
pub fn listable(path: &Path, kind: Option<FileType>) -> bool {
    let plain = |t: FileType| t.is_dir() || t.is_file();
    kind.is_some_and(plain)
        || match std::fs::metadata(path) {
            Ok(m) => plain(m.file_type()),
            Err(_) => kind.is_some_and(|t| t.is_symlink()),
        }
}

/// The entries of `dir` (relative to `root`) and whether each is a directory, unsorted. A link
/// to a directory is one.
fn read_level(root: &Path, dir: &Path) -> Vec<(PathBuf, bool)> {
    let Ok(read) = std::fs::read_dir(root.join(dir)) else {
        return Vec::new();
    };
    read.flatten()
        .filter(|e| !is_store(&e.file_name()))
        .filter(|e| listable(&e.path(), e.file_type().ok()))
        .map(|e| {
            let is_dir = e
                .file_type()
                .is_ok_and(|t| t.is_dir() || t.is_symlink() && e.path().is_dir());
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

#[cfg(test)]
pub fn from_files(files: &[PathBuf]) -> Tree {
    from_files_in(files, &Order::default())
}

pub fn from_listing(files: &[PathBuf]) -> Tree {
    from_files_in(files, &Order::of(files))
}

fn from_files_in(files: &[PathBuf], order: &Order) -> Tree {
    let mut entries: Vec<(PathBuf, bool)> = files.iter().map(|p| (p.clone(), false)).collect();
    for f in files {
        for dir in f.ancestors().skip(1) {
            if !dir.as_os_str().is_empty() && !entries.iter().any(|(p, _)| p == dir) {
                entries.push((dir.to_path_buf(), true));
            }
        }
    }
    entries.sort_by_cached_key(|(p, is_dir)| sort_key(p, *is_dir, order));
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

pub(crate) fn sort_key(path: &Path, is_dir: bool, order: &Order) -> Vec<(usize, u8, String)> {
    let last = path.components().count() - 1;
    let mut prefix = PathBuf::new();
    path.components()
        .enumerate()
        .map(|(i, c)| {
            prefix.push(c);
            let rank = u8::from(i == last && !is_dir);
            (
                order.at(&prefix),
                rank,
                c.as_os_str().to_string_lossy().to_lowercase(),
            )
        })
        .collect()
}

#[derive(Default)]
pub struct Order(HashMap<PathBuf, usize>);

pub fn orders_on() -> bool {
    !std::env::var("MERL_ORDER").is_ok_and(|v| v == "off")
}

impl Order {
    pub fn read(file: &Path, under: &Path) -> Order {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        Order::of(
            text.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .filter_map(|l| {
                    Path::new(l.trim_start_matches("./"))
                        .strip_prefix(under)
                        .ok()
                }),
        )
    }

    pub fn of<P: AsRef<Path>>(paths: impl IntoIterator<Item = P>) -> Order {
        let mut at = HashMap::new();
        for (i, path) in paths.into_iter().enumerate() {
            for a in path
                .as_ref()
                .ancestors()
                .filter(|a| !a.as_os_str().is_empty())
            {
                at.entry(a.to_path_buf()).or_insert(i);
            }
        }
        Order(at)
    }

    fn at(&self, path: &Path) -> usize {
        self.0.get(path).copied().unwrap_or(usize::MAX)
    }
}

#[derive(Clone)]
pub struct OrderFile {
    pub file: PathBuf,
    pub under: PathBuf,
}

impl OrderFile {
    pub fn read(&self) -> Order {
        Order::read(&self.file, &self.under)
    }
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
        fresh.sync_ignored();
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
        let folded: HashSet<&Path> = self
            .folded
            .iter()
            .map(|&i| self.nodes[i].path.as_path())
            .collect();
        fresh.folded = (fresh.nodes.iter().enumerate())
            .filter(|(_, n)| folded.contains(n.path.as_path()))
            .map(|(i, _)| i)
            .collect();
        *self = fresh;
    }

    /// Has an ignored directory rows below it exactly while it is expanded: an expanded one with
    /// none is read, one level; a collapsed one forgets its rows, so the next expand reads the
    /// disk again. An expanded directory among the forgotten rows is expanded again when its
    /// parent is read.
    fn sync_ignored(&mut self) {
        let len = self.nodes.len();
        let mut i = 0;
        while let Some(n) = self.nodes.get(i) {
            if !(n.ignored && n.is_dir) {
                i += 1;
                continue;
            }
            let below = self.nodes[i + 1..]
                .iter()
                .take_while(|c| c.depth > n.depth)
                .count();
            if !n.expanded && below > 0 {
                let open = self
                    .nodes
                    .drain(i + 1..i + 1 + below)
                    .filter(|c| c.expanded);
                self.away.extend(open.map(|c| c.path));
            } else if n.expanded && below == 0 {
                let mut level: Vec<Node> = read_level(&self.root, &n.path)
                    .into_iter()
                    .map(|(p, is_dir)| Node {
                        expanded: self.away.remove(&p),
                        ..node(p, is_dir, true)
                    })
                    .collect();
                level.sort_by_cached_key(|c| sort_key(&c.path, c.is_dir, &Order::default()));
                self.nodes.splice(i + 1..i + 1, level);
            }
            i += 1;
        }
        if self.nodes.len() != len {
            self.folded.clear();
        }
    }

    /// The directories the walk went into, relative to the root: not the ignored ones.
    pub fn dirs(&self) -> HashSet<PathBuf> {
        let dirs = self.nodes.iter().filter(|n| n.is_dir && !n.ignored);
        dirs.map(|n| n.path.clone()).collect()
    }

    /// The expanded ignored directories: the tree shows what they hold, so it is watched.
    pub fn open_ignored(&self) -> HashSet<PathBuf> {
        let open = self
            .nodes
            .iter()
            .filter(|n| n.ignored && n.is_dir && n.expanded);
        open.map(|n| n.path.clone()).collect()
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

    pub fn visible(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut hidden_below = usize::MAX;
        for (i, n) in self.nodes.iter().enumerate() {
            if n.depth > hidden_below {
                continue;
            }
            hidden_below = usize::MAX;
            if !self.folded.contains(&i) {
                out.push(i);
            }
            if n.is_dir && !n.expanded {
                hidden_below = n.depth;
            }
        }
        out
    }

    pub fn fold(&mut self, lacks: impl Fn(&Node, usize) -> usize) {
        self.folded.clear();
        self.lift = vec![0; self.nodes.len()];
        let mut chains = Vec::new();
        let mut top = 0;
        while top < self.nodes.len() {
            let mut head = top;
            while self.only_dir_child(head) {
                head += 1;
            }
            if head > top {
                chains.push((top, head));
            }
            top = head + 1;
        }
        for (top, head) in chains.into_iter().rev() {
            let end = self.end(head);
            let shown_below = match self.nodes[head].expanded {
                true => usize::MAX,
                false => self.nodes[head].depth + 1,
            };
            let mut hidden_below = usize::MAX;
            let mut lack = 0;
            for j in head + 1..end {
                let n = &self.nodes[j];
                if n.depth > hidden_below || n.depth > shown_below {
                    continue;
                }
                hidden_below = usize::MAX;
                if n.is_dir && !n.expanded {
                    hidden_below = n.depth;
                }
                lack = lack.max(lacks(n, n.depth - self.lift[j]));
            }
            let levels = lack.div_ceil(2).min(head - top);
            self.folded.extend(head - levels..head);
            for lift in &mut self.lift[head..end] {
                *lift += levels;
            }
        }
        while self.folded.contains(&self.cursor) {
            self.cursor += 1;
        }
    }

    pub fn drawn_depth(&self, i: usize) -> usize {
        self.nodes[i].depth - self.lift.get(i).copied().unwrap_or(0)
    }

    pub fn row_name(&self, i: usize) -> String {
        let joined = (0..i).rev().take_while(|j| self.folded.contains(j)).count();
        let parts: Vec<String> = self.nodes[i - joined..=i].iter().map(Node::name).collect();
        parts.join("/")
    }

    fn only_dir_child(&self, i: usize) -> bool {
        let n = &self.nodes[i];
        let child = self.nodes.get(i + 1);
        n.is_dir
            && !n.ignored
            && n.expanded
            && child.is_some_and(|c| c.is_dir && !c.ignored && c.depth == n.depth + 1)
            && self.end(i + 1) == self.end(i)
    }

    fn end(&self, i: usize) -> usize {
        let depth = self.nodes[i].depth;
        let below = self.nodes[i + 1..].iter().position(|n| n.depth <= depth);
        below.map_or(self.nodes.len(), |b| i + 1 + b)
    }

    fn parent(&self, i: usize) -> Option<usize> {
        let depth = self.nodes[i].depth;
        self.nodes[..i].iter().rposition(|n| n.depth + 1 == depth)
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
        self.sync_ignored();
    }

    pub fn expand(&mut self) {
        if let Some(n) = self.nodes.get_mut(self.cursor)
            && n.is_dir
        {
            n.expanded = true;
        }
        self.sync_ignored();
    }

    pub fn collapse(&mut self) {
        match self.nodes.get_mut(self.cursor) {
            Some(n) if n.is_dir && n.expanded => {
                n.expanded = false;
                self.sync_ignored();
            }
            Some(_) => {
                while let Some(p) = self.parent(self.cursor) {
                    self.cursor = p;
                    if !self.folded.contains(&p) {
                        break;
                    }
                }
            }
            None => {}
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
    fn the_project_order_moves_the_rows_and_leaves_the_file_list_alone() {
        let dir = std::env::temp_dir().join(format!("merl-tree-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for f in ["sub/a.rs", "sub/b.rs", "sub/z/c.rs", "top.rs"] {
            std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            std::fs::write(dir.join(f), "").unwrap();
        }
        let file = dir.join("order");
        std::fs::write(&file, "# notes\ntop.rs\n./sub/b.rs\nsub/z/c.rs\n").unwrap();
        let order = OrderFile {
            file,
            under: PathBuf::from("sub"),
        }
        .read();
        let (t, files) = build_ordered(&dir.join("sub"), false, &order);
        let rows: Vec<_> = t
            .nodes
            .iter()
            .map(|n| n.path.display().to_string())
            .collect();
        assert_eq!(rows, ["b.rs", "z", "z/c.rs", "a.rs"]);
        assert_eq!(files, build(&dir.join("sub"), false).1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_listing_keeps_its_order_and_the_tree_its_shape() {
        let listed =
            ["src/b/y.rs", "README", "src/a.rs", "tests/t.rs", "src/c.rs"].map(PathBuf::from);
        let t = from_listing(&listed);
        let rows: Vec<_> = t
            .visible()
            .iter()
            .map(|&i| t.nodes[i].path.to_str().unwrap())
            .collect();
        assert_eq!(
            rows,
            [
                "src",
                "src/b",
                "src/b/y.rs",
                "src/a.rs",
                "src/c.rs",
                "README",
                "tests",
                "tests/t.rs"
            ]
        );
        let order = Order::of(["b/y.rs", "a.rs"]);
        let mut paths = ["c.rs", "a.rs", "b", "b/y.rs", "b/x.rs"].map(PathBuf::from);
        paths.sort_by_cached_key(|p| sort_key(p, p.extension().is_none(), &order));
        assert_eq!(
            paths.clone().map(|p| p.display().to_string()),
            ["b", "b/y.rs", "b/x.rs", "a.rs", "c.rs"]
        );
        let mut plain = paths.clone();
        plain.sort_by_cached_key(|p| sort_key(p, p.extension().is_none(), &Order::default()));
        assert_eq!(
            plain.map(|p| p.display().to_string()),
            ["b", "b/x.rs", "b/y.rs", "a.rs", "c.rs"]
        );
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

    /// #405: opening a FIFO blocks until something writes to it, so a FIFO is not a row and not
    /// in the list the searches read, neither from the walk nor from an ignored level; a link
    /// to a regular file is, a link to a FIFO is not, and a link that leads nowhere still is.
    #[test]
    #[cfg(unix)]
    fn a_fifo_is_neither_a_row_nor_in_the_list() {
        let dir = project("fifo", &["a.py", "src/b.py", "build/out.txt"]);
        std::fs::write(dir.join(".gitignore"), "build/\n*.sock\n").unwrap();
        for f in ["pipe", "src/pipe", "srv.sock", "build/b.sock"] {
            let mkfifo = std::process::Command::new("mkfifo")
                .arg(dir.join(f))
                .status();
            assert!(mkfifo.unwrap().success(), "{f}");
        }
        std::os::unix::fs::symlink("a.py", dir.join("link.py")).unwrap();
        std::os::unix::fs::symlink("pipe", dir.join("link.pipe")).unwrap();
        // A link that leads nowhere stays a row, as it was before FIFOs were left out.
        std::os::unix::fs::symlink("missing.py", dir.join("gone.py")).unwrap();
        let (mut t, files) = build(&dir, false);
        assert_eq!(
            files,
            [
                PathBuf::from("src/b.py"),
                ".gitignore".into(),
                "a.py".into(),
                "gone.py".into(),
                "link.py".into()
            ]
        );
        assert!(t.ignored_files().is_empty());
        t.reveal(Path::new("build"));
        t.expand();
        t.reveal(Path::new("src"));
        t.expand();
        assert_eq!(
            rows(&t),
            [
                "build",
                "build/out.txt",
                "src",
                "src/b.py",
                ".gitignore",
                "a.py",
                "gone.py",
                "link.py"
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
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

        // Collapsed, it forgets its rows; expanded again, it reads the disk as it is now, and
        // what was open inside it opens again.
        t.reveal(Path::new("node_modules"));
        t.collapse();
        assert_eq!(
            rows(&t),
            ["node_modules", "src", ".env", ".gitignore", "a.py", "c.py"]
        );
        assert_eq!(
            t.nodes.len(),
            7,
            "`src/local.yml` below the collapsed `src`"
        );
        std::fs::write(dir.join("node_modules/pkg/new.js"), b"x").unwrap();
        t.expand();
        assert_eq!(
            rows(&t)[..6],
            [
                "node_modules",
                "node_modules/pkg",
                "node_modules/pkg/lib",
                "node_modules/pkg/lib/x.js",
                "node_modules/pkg/index.js",
                "node_modules/pkg/new.js",
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// #404: a symlink to a directory is a directory, read from disk when it is expanded, as an
    /// ignored one is. The walk does not follow it, so a link to `..` loops nothing and what it
    /// leads to is not in the list.
    #[cfg(unix)]
    #[test]
    fn a_link_to_a_directory_is_a_directory_the_walk_does_not_follow() {
        let dir = project("dirlink", &["docs/a.md", "z.py"]);
        std::os::unix::fs::symlink("docs", dir.join("alink")).unwrap();
        std::os::unix::fs::symlink("..", dir.join("docs/up")).unwrap();
        let (mut t, files) = build(&dir, false);
        assert_eq!(files, [PathBuf::from("docs/a.md"), "z.py".into()]);
        assert_eq!(rows(&t), ["alink", "docs", "z.py"]);
        t.reveal(Path::new("alink"));
        t.toggle();
        assert_eq!(
            rows(&t),
            ["alink", "alink/up", "alink/a.md", "docs", "z.py"]
        );
        // The link back up is a directory too, read one level when it is opened, not before.
        t.down();
        t.toggle();
        assert_eq!(
            rows(&t)[..5],
            [
                "alink",
                "alink/up",
                "alink/up/alink",
                "alink/up/docs",
                "alink/up/z.py"
            ]
        );
        assert!(t.dirs() == HashSet::from([PathBuf::from("docs")]));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn narrow(width: usize) -> impl Fn(&Node, usize) -> usize {
        move |n, depth| (2 * depth + 2 + n.name().len().min(12)).saturating_sub(width)
    }

    fn drawn(t: &Tree) -> Vec<String> {
        let vis = t.visible();
        let row = |&i: &usize| format!("{}{}", "  ".repeat(t.drawn_depth(i)), t.row_name(i));
        vis.iter().map(row).collect()
    }

    #[test]
    fn a_one_child_chain_keeps_its_rows_while_the_names_below_fit() {
        let mut t = from_files(&["a/b/c/d/x.rs".into(), "a/b/c/d/y.rs".into()]);
        t.fold(narrow(14));
        assert_eq!(
            drawn(&t),
            [
                "a",
                "  b",
                "    c",
                "      d",
                "        x.rs",
                "        y.rs"
            ]
        );
    }

    #[test]
    fn the_lowest_directories_of_a_chain_share_a_row_as_few_as_give_the_names_room() {
        let mut t = from_files(&["a/b/c/d/x.rs".into(), "a/b/c/d/y.rs".into()]);
        t.fold(narrow(10));
        assert_eq!(drawn(&t), ["a", "  b/c/d", "    x.rs", "    y.rs"]);
        t.fold(narrow(0));
        assert_eq!(drawn(&t), ["a/b/c/d", "  x.rs", "  y.rs"]);
    }

    #[test]
    fn left_and_right_work_on_a_shared_row() {
        let mut t = from_files(&["a/b/c/d/x.rs".into(), "a/b/c/d/y.rs".into()]);
        t.fold(narrow(10));
        t.down();
        assert_eq!(t.selected().unwrap().path, Path::new("a/b/c/d"));
        t.collapse();
        t.fold(narrow(10));
        assert_eq!(drawn(&t), ["a", "  b/c/d"]);
        t.collapse();
        assert_eq!(t.selected().unwrap().path, Path::new("a"));
        t.down();
        t.expand();
        t.fold(narrow(10));
        assert_eq!(drawn(&t), ["a", "  b/c/d", "    x.rs", "    y.rs"]);
        t.down();
        t.collapse();
        assert_eq!(t.selected().unwrap().path, Path::new("a/b/c/d"));
    }

    #[test]
    fn collapsing_a_high_directory_of_a_chain_leaves_the_rows_above_it_alone() {
        let mut t = from_files(&["a/b/c/d/e/x.rs".into(), "a/b/c/d/e/y.rs".into()]);
        t.fold(narrow(12));
        assert_eq!(
            drawn(&t),
            ["a", "  b", "    c/d/e", "      x.rs", "      y.rs"]
        );
        t.down();
        t.collapse();
        t.fold(narrow(12));
        assert_eq!(drawn(&t), ["a", "  b"]);
        t.expand();
        t.fold(narrow(12));
        assert_eq!(
            drawn(&t),
            ["a", "  b", "    c/d/e", "      x.rs", "      y.rs"]
        );
    }

    #[test]
    fn an_odd_lack_folds_one_more_level() {
        let mut t = from_files(&["a/b/c/d/x.rs".into()]);
        t.fold(narrow(11));
        assert_eq!(drawn(&t), ["a", "  b/c/d", "    x.rs"]);
    }

    #[test]
    fn a_lower_chain_folds_before_the_one_above_it() {
        let mut t = from_files(&["a/b/c/d/e/x.rs".into(), "a/b/z.rs".into()]);
        t.fold(narrow(14));
        assert_eq!(
            drawn(&t),
            ["a", "  b", "    c", "      d/e", "        x.rs", "    z.rs"]
        );
    }

    #[test]
    fn a_refresh_keeps_the_rows_folded_until_the_next_draw() {
        let dir = project("refold", &["a/b/c/d/x.rs", "z.rs"]);
        let (mut t, _) = build(&dir, false);
        t.reveal(Path::new("a/b/c/d/x.rs"));
        t.fold(narrow(10));
        let before = drawn(&t);
        t.refresh(build(&dir, false).0);
        assert_eq!(t.visible().len(), before.len());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_directory_with_two_children_ends_the_chain_and_its_children_never_fold() {
        let mut t = from_files(&["a/b/c/e/x.rs".into(), "a/b/d/f/y.rs".into()]);
        t.fold(narrow(0));
        assert_eq!(drawn(&t), ["a/b", "  c/e", "    x.rs", "  d/f", "    y.rs"]);
    }

    #[test]
    fn the_cursor_never_rests_on_a_folded_directory() {
        let mut t = from_files(&["a/b/c/d/x.rs".into()]);
        t.fold(narrow(0));
        assert_eq!(t.selected().unwrap().path, Path::new("a/b/c/d"));
    }

    #[test]
    fn an_ignored_directory_stays_a_row_of_its_own() {
        let dir = project("chain", &[".gitignore", "a/b/gen/deep/x.py"]);
        std::fs::write(dir.join(".gitignore"), "gen/\n").unwrap();
        let (mut t, _) = build(&dir, false);
        t.reveal(Path::new("a/b/gen"));
        t.expand();
        t.down();
        t.expand();
        t.fold(narrow(0));
        assert_eq!(
            drawn(&t),
            ["a/b", "  gen", "    deep", "      x.py", ".gitignore"]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
