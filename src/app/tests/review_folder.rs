use super::*;
use std::time::{Duration, SystemTime};

struct Clones {
    dir: PathBuf,
}

impl Clones {
    fn new(tag: &str) -> Self {
        let tmp = std::env::temp_dir().canonicalize().unwrap();
        let dir = tmp.join(format!("merl-folder-{tag}-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let c = Self { dir };
        c.git("", &["init", "-q", "--bare", "-b", "main", "remote.git"]);
        c.git("", &["clone", "-q", "remote.git", "a"]);
        c.git("a", &["switch", "-q", "-c", "main"]);
        c.commit("a", ".gitignore", ".venv/\n");
        c.commit("a", "README", "readme\n");
        c.git("a", &["push", "-q", "origin", "main"]);
        for b in ["feat1", "feat2", "feat3"] {
            c.git("a", &["switch", "-q", "-c", b, "main"]);
            c.commit("a", &format!("{b}.py"), "x = 1\n");
            c.git("a", &["push", "-q", "origin", b]);
        }
        c.git("", &["clone", "-q", "remote.git", "b"]);
        c.git("b", &["switch", "-q", "-c", "mine"]);
        std::fs::write(c.b().join("README"), "my edit\n").unwrap();
        std::fs::create_dir_all(c.b().join(".venv/lib")).unwrap();
        std::fs::write(c.b().join(".venv/lib/pkg.py"), "def f(): pass\n").unwrap();
        c
    }

    fn b(&self) -> PathBuf {
        self.dir.join("b")
    }

    fn git(&self, at: &str, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .current_dir(self.dir.join(at))
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    fn commit(&self, at: &str, file: &str, text: &str) {
        std::fs::write(self.dir.join(at).join(file), text).unwrap();
        self.git(at, &["add", "."]);
        self.git(at, &["commit", "-q", "-m", file]);
    }

    fn open(&self, branch: &str) -> (PathBuf, git::Review) {
        git::Review::open_branch(&self.b(), branch, None).unwrap()
    }
}

impl Drop for Clones {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.dir));
    }
}

fn names(r: &git::Review) -> Vec<String> {
    (r.files.iter())
        .map(|f| f.path.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn two_reviews_of_pushed_branches_keep_their_own_folders_and_your_checkout() {
    let c = Clones::new("two");
    let (one, r1) = c.open("feat1");
    let (two, r2) = c.open("origin/feat2");
    assert_ne!(one, two);
    assert_eq!(one.file_name(), Some(std::ffi::OsStr::new("b")));
    assert_eq!(
        (
            r1.branch.as_str(),
            names(&r1),
            r2.branch.as_str(),
            names(&r2)
        ),
        (
            "feat1",
            vec!["feat1.py".to_owned()],
            "feat2",
            vec!["feat2.py".to_owned()]
        )
    );
    let again = r1.refresh(&one).unwrap();
    assert_eq!(
        (again.branch.as_str(), names(&again)),
        ("feat1", vec!["feat1.py".to_owned()]),
        "the first tab's folder still holds its branch after the second review"
    );
    assert_eq!(c.git("b", &["symbolic-ref", "--short", "HEAD"]), "mine\n");
    assert_eq!(c.git("b", &["status", "--porcelain"]), " M README\n");
    assert_eq!(
        std::fs::read_link(one.join(".venv")).unwrap(),
        c.b().join(".venv"),
        "`d` into libraries reads your checkout's .venv"
    );

    c.git("a", &["switch", "-q", "feat1"]);
    c.commit("a", "fix.py", "y = 2\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    let (reused, r1) = c.open("feat1");
    assert_eq!(reused, one);
    assert_eq!(
        names(&r1),
        ["feat1.py", "fix.py"],
        "brought up to what was pushed"
    );
    assert_eq!(
        crate::app::notes::store(&git::dirs(&one).unwrap()).map(|(_, b)| b),
        Some("feat1".to_owned()),
        "notes keep the branch's name in a detached folder"
    );

    let mut a = App::new(
        one.clone(),
        crate::tree::from_listing(&[PathBuf::from("fix.py")]),
        Vec::new(),
        Buffer::load(&one.join("fix.py")).unwrap(),
        None,
    );
    a.pushed_folder = git::Opened::mark(&one);
    assert!(a.pushed_folder.is_some());
    a.start_review(r1);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.mode, a.message.as_str()),
        (Mode::Normal, "read-only: a pushed branch")
    );
    press(&mut a, KeyCode::Char('n'), KeyModifiers::CONTROL);
    typed(&mut a, "new.py");
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.message, "read-only: a pushed branch");
    assert!(!one.join("new.py").exists(), "Ctrl+N writes nothing");
}

#[test]
fn a_folder_keeps_your_own_commits_and_opens_offline_at_the_last_fetch() {
    let c = Clones::new("own");
    c.git("b", &["switch", "-q", "feat2"]);
    std::fs::write(c.b().join("mine.py"), "z = 3\n").unwrap();
    c.git("b", &["add", "mine.py"]);
    c.git("b", &["commit", "-q", "-m", "mine"]);
    c.git("b", &["switch", "-q", "mine"]);
    c.git("a", &["switch", "-q", "feat2"]);
    c.commit("a", "theirs.py", "w = 4\n");
    c.git("a", &["push", "-q", "origin", "feat2"]);
    let (_, r) = c.open("feat2");
    assert_eq!(
        (names(&r), r.note.as_deref()),
        (
            vec!["feat2.py".to_owned(), "mine.py".to_owned()],
            Some("diverged from origin/feat2")
        ),
        "your own commit stays, and the review says origin moved on"
    );

    std::fs::remove_dir_all(c.dir.join("remote.git")).unwrap();
    let (_, r) = c.open("feat3");
    assert_eq!(
        (names(&r), r.note.as_deref()),
        (
            vec!["feat3.py".to_owned()],
            Some("origin/feat3 not fetched")
        ),
        "offline, a branch with no local copy opens at what was fetched last"
    );
}

#[test]
fn a_sweep_removes_aged_and_gone_folders_but_never_an_open_one() {
    let c = Clones::new("sweep");
    let (one, _) = c.open("feat1");
    let (two, _) = c.open("feat2");
    let (three, _) = c.open("feat3");
    let open = git::Opened::mark(&one);
    for f in [&two, &three] {
        drop(git::Opened::mark(f));
    }
    c.git("a", &["push", "-q", "origin", "--delete", "feat3"]);
    let now = SystemTime::now();
    git::sweep_folders(&c.b(), now);
    assert_eq!(
        (one.exists(), two.exists(), three.exists()),
        (true, true, false),
        "a branch gone from origin"
    );
    git::sweep_folders(&c.b(), now + Duration::from_hours(15 * 24));
    assert_eq!(
        (one.exists(), two.exists()),
        (true, false),
        "unopened for 15 days, but the open one stays"
    );
    drop(open);
    git::sweep_folders(&c.b(), now + Duration::from_hours(15 * 24));
    assert!(!one.exists());
    assert!(c.b().join(".venv/lib/pkg.py").is_file(), "your .venv stays");
    let common = c.b().join(".git/merl/branches");
    assert_eq!(std::fs::read_dir(common).unwrap().count(), 0);
    assert_eq!(
        c.git("b", &["worktree", "list", "--porcelain"])
            .matches("worktree ")
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn a_folder_s_workspace_packages_are_its_own_and_only_dependencies_are_yours() {
    let c = Clones::new("workspace");
    let write = |at: &Path, f: &str, text: &str| {
        std::fs::create_dir_all(at.join(f).parent().unwrap()).unwrap();
        std::fs::write(at.join(f), text).unwrap();
    };
    let link = |to: &str, at: PathBuf| {
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(to, at).unwrap();
    };
    let a = c.dir.join("a");
    c.git("a", &["switch", "-q", "feat1"]);
    write(&a, "packages/shared/index.ts", "theirs\n");
    write(&a, "packages/web/package.json", "{}\n");
    c.commit("a", ".gitignore", ".venv/\nnode_modules/\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    let b = c.b();
    write(&b, "packages/shared/index.ts", "mine\n");
    write(&b, "node_modules/lib/index.js", "lib\n");
    link("../../packages/shared", b.join("node_modules/@app/shared"));
    link(
        "../../../shared",
        b.join("packages/web/node_modules/@app/shared"),
    );
    link(
        "../../../node_modules/lib",
        b.join("packages/web/node_modules/lib"),
    );
    let (folder, _) = c.open("feat1");
    let read = |f: &str| std::fs::read_to_string(folder.join(f)).unwrap_or_default();
    let files = [
        "node_modules/@app/shared/index.ts",
        "packages/web/node_modules/@app/shared/index.ts",
        "node_modules/lib/index.js",
        "packages/web/node_modules/lib/index.js",
        "node_modules/added/index.js",
    ];
    assert_eq!(
        files.map(read),
        ["theirs\n", "theirs\n", "lib\n", "lib\n", ""],
        "a workspace package is the reviewed branch's, a dependency your checkout's"
    );
    write(&b, "node_modules/added/index.js", "added\n");
    assert_eq!(c.open("feat1").0, folder);
    assert_eq!(
        files.map(read),
        ["theirs\n", "theirs\n", "lib\n", "lib\n", "added\n"],
        "a reused folder keeps its links and gets the ones you installed since"
    );
}
