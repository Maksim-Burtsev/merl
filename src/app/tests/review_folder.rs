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
    std::fs::write(one.join("feat1.py"), "x = 2\n").unwrap();
    std::fs::write(one.join("new.py"), "y = 1\n").unwrap();
    git::sweep_folders(&c.b(), now + Duration::from_hours(15 * 24));
    assert_eq!(
        (
            std::fs::read_to_string(one.join("feat1.py")).ok(),
            one.join("new.py").is_file(),
            std::fs::read_link(one.join(".venv")).ok()
        ),
        (Some("x = 2\n".to_owned()), true, Some(c.b().join(".venv"))),
        "an aged folder with your edits stays, edits and links and all"
    );
    c.git(
        "b",
        &["-C", one.to_str().unwrap(), "checkout", "--", "feat1.py"],
    );
    std::fs::remove_file(one.join("new.py")).unwrap();
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
fn a_sweep_takes_a_package_s_node_modules_link_but_not_what_it_points_to() {
    let c = Clones::new("sweep-package");
    c.git("a", &["switch", "-q", "feat1"]);
    std::fs::create_dir_all(c.dir.join("a/packages/web")).unwrap();
    c.commit("a", "packages/web/package.json", "{}\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    let mine = c.b().join("packages/web/node_modules");
    std::fs::create_dir_all(mine.join("lib")).unwrap();
    let (folder, _) = c.open("feat1");
    assert!(folder.join("packages/web/node_modules").is_symlink());
    git::sweep_folders(&c.b(), SystemTime::now() + Duration::from_hours(15 * 24));
    assert_eq!(
        (folder.exists(), mine.join("lib").is_dir()),
        (false, true),
        "the link alone does not keep the folder, and your node_modules stays"
    );
}

#[cfg(unix)]
#[test]
fn d_through_a_folder_s_node_modules_lands_in_its_own_workspace_package() {
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
    write(&a, "packages/shared/Token.sol", "contract Theirs {}\n");
    write(&a, "packages/shared/theme.css", ".theirs {}\n");
    write(&a, "packages/web/package.json", "{}\n");
    let app = "import \"@app/shared/Token.sol\";\nimport \"lib/Lib.sol\";\n";
    write(&a, "packages/web/App.sol", app);
    write(
        &a,
        "packages/web/app.css",
        "@import \"@app/shared/theme.css\";\n",
    );
    c.commit("a", ".gitignore", ".venv/\nnode_modules/\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    let b = c.b();
    write(&b, "packages/shared/Token.sol", "contract Mine {}\n");
    write(&b, "packages/shared/theme.css", ".mine {}\n");
    write(&b, "node_modules/lib/Lib.sol", "contract Lib {}\n");
    link(
        "../../../shared",
        b.join("packages/web/node_modules/@app/shared"),
    );
    let (folder, _) = c.open("feat1");
    let mut a = App::new(
        folder.clone(),
        crate::tree::from_listing(&[PathBuf::from("packages/web/app.css")]),
        Vec::new(),
        Buffer::load(&folder.join("packages/web/app.css")).unwrap(),
        None,
    );
    let place = |a: &mut App, file: &str, code: &str| {
        d_on(a, file, code);
        match shown(a) {
            Shown::Jump(_, place) => place,
            Shown::Picker(status, _) => status,
        }
    };
    assert_eq!(
        [
            place(&mut a, "packages/web/App.sol", "Token"),
            place(&mut a, "packages/web/app.css", "theme"),
            place(&mut a, "packages/web/App.sol", "Lib.sol"),
        ],
        [
            "packages/shared/Token.sol:1",
            "packages/shared/theme.css:1",
            "node_modules/lib/Lib.sol:1"
        ],
        "a workspace package is the reviewed branch's, a dependency your checkout's"
    );
    write(&b, "packages/web/App.sol", app);
    let mut yours = App::new(
        b.clone(),
        crate::tree::from_listing(&[PathBuf::from("packages/web/App.sol")]),
        Vec::new(),
        Buffer::load(&b.join("packages/web/App.sol")).unwrap(),
        None,
    );
    assert_eq!(
        place(&mut yours, "packages/web/App.sol", "Token"),
        "packages/web/node_modules/@app/shared/Token.sol:1",
        "your own checkout reads its workspace link as before"
    );

    c.git("a", &["rm", "-q", "-r", "packages/web"]);
    c.git("a", &["commit", "-q", "-m", "drop web"]);
    c.git("a", &["push", "-q", "origin", "feat1"]);
    std::fs::rename(b.join(".venv"), b.join("venv.moved")).unwrap();
    let (again, r) = c.open("feat1");
    assert_eq!(
        (again.join("packages/web").exists(), names(&r)),
        (
            false,
            vec![
                "packages/shared/theme.css".to_owned(),
                "packages/shared/Token.sol".to_owned(),
                ".gitignore".to_owned(),
                "feat1.py".to_owned(),
            ]
        ),
        "a package the branch drops leaves no folder, a link to nothing no row"
    );
}

#[test]
fn a_name_that_climbs_out_of_the_folders_deletes_nothing() {
    let c = Clones::new("climb");
    let kept = c.b().join("b/keep.py");
    std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
    std::fs::write(&kept, "x = 1\n").unwrap();
    for name in [c.b().to_str().unwrap(), "../../..", "feat1/../../../.."] {
        let e = git::Review::open_branch(&c.b(), name, None).unwrap_err();
        assert!(format!("{e:#}").contains("no branch"), "{name}: {e:#}");
        assert!(kept.is_file(), "{name} deleted the checkout's b/keep.py");
    }
}

#[test]
fn opening_a_review_again_keeps_your_edits_in_its_folder_and_says_it_is_behind() {
    let c = Clones::new("edits");
    let (one, _) = c.open("feat1");
    std::fs::write(one.join("feat1.py"), "x = 2\n").unwrap();
    std::fs::write(one.join("new.py"), "y = 1\n").unwrap();
    c.git("a", &["switch", "-q", "feat1"]);
    c.commit("a", "fix.py", "y = 2\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    let (again, r) = c.open("feat1");
    assert_eq!(again, one);
    assert_eq!(
        (
            std::fs::read_to_string(one.join("feat1.py")).ok(),
            one.join("new.py").is_file(),
            one.join("fix.py").exists(),
            r.note.as_deref()
        ),
        (
            Some("x = 2\n".to_owned()),
            true,
            false,
            Some("behind origin/feat1")
        ),
        "your edits stay, and the folder stays at its commit"
    );

    c.git(
        "b",
        &["-C", one.to_str().unwrap(), "checkout", "--", "feat1.py"],
    );
    std::fs::remove_file(one.join("new.py")).unwrap();
    let (_, r) = c.open("feat1");
    assert_eq!(
        (names(&r), r.note),
        (vec!["feat1.py".to_owned(), "fix.py".to_owned()], None),
        "with no edits, past the links merl made, it moves to what was pushed"
    );

    c.git("b", &["config", "status.showUntrackedFiles", "no"]);
    std::fs::write(one.join("later.py"), "mine\n").unwrap();
    c.commit("a", "later.py", "theirs\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    let (_, r) = c.open("feat1");
    assert_eq!(
        (
            std::fs::read_to_string(one.join("later.py")).ok(),
            r.note.as_deref()
        ),
        (Some("mine\n".to_owned()), Some("behind origin/feat1")),
        "a new file is an edit whatever status.showUntrackedFiles says"
    );
}

fn commit_in_a_folder(c: &Clones) -> (String, String) {
    let (one, _) = c.open("feat1");
    let at = one.to_str().unwrap().to_owned();
    std::fs::write(one.join("mine.py"), "z = 3\n").unwrap();
    c.git("b", &["-C", &at, "add", "mine.py"]);
    c.git("b", &["-C", &at, "commit", "-q", "-m", "mine"]);
    let mine = c.git("b", &["-C", &at, "rev-parse", "HEAD"]);
    (at, mine)
}

#[test]
fn a_commit_made_in_a_review_folder_stays_there_when_the_review_opens_again() {
    let c = Clones::new("folder-commit");
    let (at, mine) = commit_in_a_folder(&c);
    c.git("b", &["-C", &at, "checkout", "-q", "HEAD~1"]);
    c.git("b", &["-C", &at, "checkout", "-q", "-"]);
    let (_, r) = c.open("feat1");
    assert_eq!(
        (c.git("b", &["-C", &at, "rev-parse", "HEAD"]), r.note),
        (mine, None),
        "with nothing new on origin, the folder stays at your commit"
    );
}

#[test]
fn a_commit_made_in_a_review_folder_stays_there_after_a_push_to_the_branch() {
    let c = Clones::new("folder-commit-push");
    let (at, mine) = commit_in_a_folder(&c);
    c.git("a", &["switch", "-q", "feat1"]);
    c.commit("a", "fix.py", "y = 2\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    let (_, r) = c.open("feat1");
    assert_eq!(
        (
            c.git("b", &["-C", &at, "rev-parse", "HEAD"]),
            names(&r),
            r.note.as_deref()
        ),
        (
            mine,
            vec!["feat1.py".to_owned(), "mine.py".to_owned()],
            Some("diverged from origin/feat1")
        ),
        "your commit stays, and the review says origin moved on"
    );
}

#[test]
fn a_review_folder_at_a_tip_that_was_force_pushed_away_moves_to_the_new_one() {
    let c = Clones::new("force-push");
    let (one, _) = c.open("feat1");
    c.git("a", &["switch", "-q", "feat1"]);
    std::fs::write(c.dir.join("a/feat1.py"), "x = 2\n").unwrap();
    c.git("a", &["commit", "-q", "-a", "--amend", "-m", "feat1"]);
    c.git("a", &["push", "-q", "-f", "origin", "feat1"]);
    let (_, r) = c.open("feat1");
    assert_eq!(
        (std::fs::read_to_string(one.join("feat1.py")).ok(), r.note),
        (Some("x = 2\n".to_owned()), None),
        "a tip origin once had is not yours to keep"
    );
}

fn commit_on(c: &Clones, branch: &str, from: &str, file: &str) {
    c.git("b", &["switch", "-q", "-c", branch, from]);
    std::fs::write(c.b().join(file), "a = 1\n").unwrap();
    c.git("b", &["add", file]);
    c.git("b", &["commit", "-q", "-m", file]);
    c.git("b", &["switch", "-q", "mine"]);
}

#[test]
fn a_branch_tip_written_without_a_message_keeps_your_own_commits() {
    let c = Clones::new("update-ref");
    commit_on(&c, "feat2", "origin/feat2", "mine.py");
    let tree = c.git("b", &["rev-parse", "feat2^{tree}"]);
    let again = c.git(
        "b",
        &["commit-tree", tree.trim(), "-p", "feat2", "-m", "again"],
    );
    c.git("b", &["update-ref", "refs/heads/feat2", again.trim()]);
    let (_, r) = c.open("feat2");
    assert_eq!(names(&r), ["feat2.py", "mine.py"]);
}

#[test]
fn a_review_folder_of_a_branch_never_pushed_follows_it_after_an_amend() {
    let c = Clones::new("local-only");
    commit_on(&c, "lfeat", "origin/main", "l.py");
    let (one, _) = c.open("lfeat");
    let tree = c.git("b", &["rev-parse", "lfeat^{tree}"]);
    let amended = c.git(
        "b",
        &["commit-tree", tree.trim(), "-p", "main", "-m", "amended"],
    );
    c.git("b", &["branch", "-q", "-f", "lfeat", amended.trim()]);
    let (_, r) = c.open("lfeat");
    assert_eq!(
        (
            c.git("b", &["-C", one.to_str().unwrap(), "rev-parse", "HEAD"]),
            r.note.as_deref()
        ),
        (amended, Some("origin/lfeat not fetched"))
    );
}

#[test]
fn a_review_folder_merl_moved_to_your_branch_follows_origin_once_that_branch_is_gone() {
    let c = Clones::new("moved");
    let (one, _) = c.open("feat1");
    commit_on(&c, "feat1", "origin/feat1", "l.py");
    c.open("feat1");
    assert!(one.join("l.py").is_file(), "the folder shows your branch");
    c.git("b", &["branch", "-q", "-D", "feat1"]);
    let (_, r) = c.open("feat1");
    assert_eq!((one.join("l.py").exists(), r.note), (false, None));
}

#[cfg(unix)]
#[test]
fn d_into_a_php_vendor_lib_from_a_folder_lands_as_in_your_checkout_and_a_sweep_keeps_your_libs() {
    let c = Clones::new("vendor");
    let user = "<?php\n\nuse Acme\\Client;\n\nfunction run()\n{\n    return new Client();\n}\n";
    let write = |at: &Path, f: &str, text: &str| {
        std::fs::create_dir_all(at.join(f).parent().unwrap()).unwrap();
        std::fs::write(at.join(f), text).unwrap();
    };
    let a = c.dir.join("a");
    c.git("a", &["switch", "-q", "feat1"]);
    write(&a, "app/Use.php", user);
    c.commit("a", ".gitignore", ".venv/\nvendor/\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    c.git("a", &["switch", "-q", "feat2"]);
    write(
        &a,
        "vendor/acme/Client.php",
        "<?php\n\nclass Theirs\n{\n}\n",
    );
    c.commit("a", "Cargo.lock", "theirs\n");
    c.git("a", &["push", "-q", "origin", "feat2"]);
    let b = c.b();
    let lib = "<?php\n\nnamespace Acme;\n\nclass Client\n{\n}\n";
    write(&b, "vendor/acme/Client.php", lib);
    write(&b, "Cargo.lock", "mine\n");
    write(&b, ".gitignore", ".venv/\nvendor/\n");
    write(&b, "app/Use.php", user);
    let land = |root: &Path| {
        let (tree, files) = crate::tree::build(root, false);
        let mut a = App::new(root.to_path_buf(), tree, files, Buffer::empty(), None);
        d_on(&mut a, "app/Use.php", "new Client");
        shown(&mut a)
    };
    let (folder, _) = c.open("feat1");
    let (tracked, _) = c.open("feat2");
    let yours = land(&b);
    assert!(
        matches!(&yours, Shown::Jump(_, at) if at == "vendor/acme/Client.php:5"),
        "{yours:?}"
    );
    assert_eq!(
        (
            land(&folder),
            std::fs::read_to_string(folder.join("Cargo.lock")).ok()
        ),
        (yours, Some("mine\n".to_owned())),
        "a review folder reads your checkout's vendor/ and untracked Cargo.lock"
    );
    assert_eq!(
        (
            tracked.join("vendor").is_symlink(),
            std::fs::read_to_string(tracked.join("vendor/acme/Client.php")).ok(),
            std::fs::read_to_string(tracked.join("Cargo.lock")).ok()
        ),
        (
            false,
            Some("<?php\n\nclass Theirs\n{\n}\n".to_owned()),
            Some("theirs\n".to_owned())
        ),
        "a vendor/ and a Cargo.lock the branch tracks are its own"
    );
    git::sweep_folders(&b, SystemTime::now() + Duration::from_hours(15 * 24));
    assert_eq!(
        (
            folder.exists(),
            tracked.exists(),
            std::fs::read_to_string(b.join("vendor/acme/Client.php")).ok(),
            std::fs::read_to_string(b.join("Cargo.lock")).ok()
        ),
        (
            false,
            false,
            Some(lib.to_owned()),
            Some("mine\n".to_owned())
        ),
        "the sweep takes the links, not your vendor/ and Cargo.lock"
    );
}

#[test]
fn a_tracked_cargo_lock_changed_in_a_review_folder_is_an_edit_it_keeps() {
    let c = Clones::new("lock");
    c.git("a", &["switch", "-q", "feat1"]);
    c.commit("a", "Cargo.lock", "# theirs\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    std::fs::write(c.b().join("Cargo.lock"), "# mine\n").unwrap();
    let (one, _) = c.open("feat1");
    assert!(
        !one.join("Cargo.lock").is_symlink(),
        "a tracked file is never linked"
    );
    std::fs::write(one.join("Cargo.lock"), "# rebuilt\n").unwrap();
    c.commit("a", "fix.py", "y = 2\n");
    c.git("a", &["push", "-q", "origin", "feat1"]);
    let (_, r) = c.open("feat1");
    assert_eq!(
        (
            std::fs::read_to_string(one.join("Cargo.lock")).ok(),
            r.note.as_deref()
        ),
        (Some("# rebuilt\n".to_owned()), Some("behind origin/feat1")),
        "the changed Cargo.lock holds the folder at its commit"
    );
}
