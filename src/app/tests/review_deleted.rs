use super::*;
use TextLine::{Deleted, File};

const REPO: &str = "\
class OrderRepository:
    def get(self, order_id):
        return self.db.get(order_id)

    def get_order_with_items(self, order_id):
        order = self.get(order_id)
        return order
";

const SERVICE: &str = "\
from app.repositories.orders import OrderRepository


class OrderService:
    def __init__(self, repo: OrderRepository):
        self.repo = repo

    def show(self, order_id):
        self.total(order_id)
        return self.repo.get_order_with_items(order_id)

    def total(self, order_id):
        return self.repo.get(order_id)


def fmt(order):
    return str(order)


def show_all(orders):
    return [fmt(o) for o in orders]
";

/// The issue's branch: the service stops calling `get_order_with_items`, the repository loses
/// it, and `app/legacy.py`, which declared a `get` of its own, is deleted. The service's `fmt`
/// moves to `app/format.py`. Nothing outside the project is searched.
fn orders_review(tag: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-440-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("app/repositories")).unwrap();
    std::fs::create_dir_all(dir.join("app/services")).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    let write = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
    write("app/repositories/orders.py", REPO);
    write("app/services/orders.py", SERVICE);
    write("app/legacy.py", "def get(key):\n    return key\n");
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    write(
        "app/repositories/orders.py",
        &REPO
            .lines()
            .take(3)
            .map(|l| format!("{l}\n"))
            .collect::<String>(),
    );
    write(
        "app/services/orders.py",
        &SERVICE
            .replace("        self.total(order_id)\n", "")
            .replace(
                "self.repo.get_order_with_items(order_id)",
                "self.base_repo.get(Order, order_id, load=[\"items\"])",
            )
            .lines()
            .take(12)
            .map(|l| format!("{l}\n"))
            .collect::<String>(),
    );
    write("app/format.py", "def fmt(order):\n    return repr(order)\n");
    std::fs::remove_file(dir.join("app/legacy.py")).unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "work"]);
    let review = git::Review::open(&dir, None, None).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir.clone(), Tree::default(), files, Buffer::empty(), None);
    a.start_review(review);
    use_roots(&mut a, Kind::Python, &[]);
    (dir, a)
}

fn on_deleted(a: &mut App, file: &Path, line: &str, word: &str) {
    a.jump_to(file, 1);
    while !a.line_str().contains(line) || a.deleted.is_none() {
        assert!(a.next_line(a.at()).is_some(), "no deleted `{line}`");
        press(a, KeyCode::Down, KeyModifiers::NONE);
    }
    a.col = a.line_str().find(word).unwrap();
}

fn rows(a: &mut App) -> Vec<String> {
    let p = a.picker.as_mut().expect("a picker");
    p.settle();
    p.window(50)
        .0
        .iter()
        .map(|r| r.item.label.clone())
        .collect()
}

#[test]
fn d_on_a_deleted_call_lands_on_the_deleted_definition() {
    let (dir, mut a) = orders_review("d-deleted");
    let service = dir.join("app/services/orders.py");
    on_deleted(
        &mut a,
        &service,
        "get_order_with_items",
        "get_order_with_items",
    );
    let from = a.at();
    assert_eq!(from, Deleted(8, 1));
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.buf.path.as_deref(),
        Some(&*dir.join("app/repositories/orders.py"))
    );
    assert_eq!(
        a.at(),
        Deleted(3, 1),
        "the deleted definition, red: {}",
        a.message
    );
    assert_eq!(
        a.line_str(),
        "    def get_order_with_items(self, order_id):"
    );
    assert_eq!(&a.line_str()[a.col..a.col + 3], "get");
    assert_eq!(
        a.message,
        "get_order_with_items \u{2192} OrderRepository.get_order_with_items (via self.repo: \
         OrderRepository)"
    );
    press(&mut a, KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at()),
        (Some(&*service), from),
        "`[` goes back to the call"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn d_prefers_the_branch_over_a_deleted_namesake() {
    let (dir, mut a) = orders_review("d-branch");
    let repo = dir.join("app/repositories/orders.py");
    a.jump_to(&dir.join("app/services/orders.py"), 12);
    a.col = a.line_str().rfind("get").unwrap();
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at()),
        (Some(&*repo), File(1)),
        "the branch's `get`, though the deleted `app/legacy.py` declares one too: {}",
        a.message
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn d_on_a_deleted_line_answers_from_the_base() {
    let (dir, mut a) = orders_review("d-base");
    let repo = dir.join("app/repositories/orders.py");
    on_deleted(&mut a, &repo, "order = self.get", "get(");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.at(), a.picker.is_none()),
        (File(1), true),
        "`self.get` on the repository's deleted line: {}",
        a.message
    );
    let service = dir.join("app/services/orders.py");
    on_deleted(&mut a, &service, "[fmt(o)", "fmt");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.line_str()),
        (Some(&*service), "def fmt(order):"),
        "the call of `fmt` is deleted with it, and the branch's `fmt` in `app/format.py` says \
         `repr`: the answer is the `fmt` the call called: {}",
        a.message
    );
    assert!(a.deleted.is_some());
    on_deleted(&mut a, &service, "self.total", "total");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at(), a.line_str()),
        (Some(&*service), File(10), "    def total(self, order_id):"),
        "`self.total` on a deleted line: the method under the block, on its line of the file: {}",
        a.message
    );
    on_deleted(&mut a, &repo, "return order", "order");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.at(),
        Deleted(3, 2),
        "`order` is bound by the deleted line above it, in the block deleted with it: {}",
        a.message
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn s_lists_deleted_lines_and_enter_lands_on_one() {
    let (dir, mut a) = orders_review("s-deleted");
    a.jump_to(&dir.join("app/services/orders.py"), 1);
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "get_order_with_items");
    a.settle_search();
    assert_eq!(
        rows(&mut a),
        [
            "app/services/orders.py:10: return self.repo.get_order_with_items(order_id)",
            "app/repositories/orders.py:5: def get_order_with_items(self, order_id):",
        ],
        "deleted lines numbered as the base had them"
    );
    assert!(a.picker.as_ref().unwrap().current().unwrap().deleted);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at()),
        (
            Some(&*dir.join("app/repositories/orders.py")),
            Deleted(3, 1)
        ),
        "Enter lands on the deleted line"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn s_keeps_its_row_apart_from_a_deleted_line_of_the_same_number() {
    let (dir, mut a) = orders_review("s-same-number");
    a.jump_to(&dir.join("app/services/orders.py"), 1);
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "self");
    a.settle_search();
    let row = "app/services/orders.py:9: ";
    while !a
        .picker
        .as_ref()
        .unwrap()
        .current()
        .unwrap()
        .label
        .starts_with(row)
        || a.picker.as_ref().unwrap().current().unwrap().deleted
    {
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    }
    typed(&mut a, ".");
    a.settle_search();
    let cur = a.picker.as_ref().unwrap().current().unwrap();
    assert!(
        cur.label.starts_with(row) && !cur.deleted,
        "the row stays, not the deleted line 9 drawn above it: {}",
        cur.label
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn u_and_capital_d_list_deleted_lines() {
    let (dir, mut a) = orders_review("u-deleted");
    let repo = dir.join("app/repositories/orders.py");
    a.jump_to(&repo, 2);
    a.col = a.line_str().find("get").unwrap();
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    let listed = rows(&mut a);
    let has = |row: &str| listed.iter().any(|r| r.contains(row));
    for row in [
        "app/repositories/orders.py:2: def get(self, order_id):",
        "app/repositories/orders.py:6: order = self.get(order_id)",
        "app/services/orders.py:12: return self.repo.get(order_id)",
    ] {
        assert!(has(row), "{row} in {listed:#?}");
    }
    let legacy = listed
        .iter()
        .position(|r| r.contains("app/legacy.py:1: def get(key):"))
        .expect("the deleted declaration");
    let title = &a.picker.as_ref().unwrap().title;
    let declarations: usize = title
        .split_once(": ")
        .and_then(|(_, counts)| counts.split_once(" declaration"))
        .map_or(0, |(n, _)| n.parse().unwrap());
    assert!(legacy < declarations, "{title}: {listed:#?}");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);

    on_deleted(
        &mut a,
        &dir.join("app/services/orders.py"),
        "get_order_with_items",
        "get_order_with_items",
    );
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    assert_eq!(
        rows(&mut a).len(),
        2,
        "from the deleted call, its uses: both deleted: {}",
        a.message
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);

    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    let listed = rows(&mut a);
    assert!(
        listed.iter().any(|r| r.starts_with("get_order_with_items")
            && r.ends_with("app/repositories/orders.py:5")),
        "{listed:#?}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_deleted_row_carries_a_red_mark() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    let (dir, mut a) = orders_review("s-red");
    a.show_tree = false;
    a.jump_to(&dir.join("app/services/orders.py"), 1);
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "self.total");
    a.settle_search();
    a.picker.as_mut().unwrap().settle();
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut t = Terminal::new(TestBackend::new(90, 12)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut a, &theme)).unwrap();
    let buf = t.backend().buffer();
    let label = "  9  self.total(order_id)";
    let (x, y) = (0..buf.area.height)
        .find_map(|y| {
            let text: String = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
            let at = text.find(label)?;
            Some((text[..at].chars().count() as u16, y))
        })
        .expect("the deleted row");
    assert_eq!(
        (buf[(x - 1, y)].symbol(), buf[(x - 1, y)].fg),
        ("\u{258e}", Color::Red),
        "the gutter's mark, red"
    );
    assert_eq!(buf[(x, y)].bg, theme.selection, "selected");
    let _ = std::fs::remove_dir_all(dir);
}

const LOADS: &str = "\
class Repo:
    def load(self, key):
        row = self.db.get(key)
        if row is None:
            raise KeyError(key)
        return row

    def fetch(self, key):
        return self.db.fetch(key)
";

const SERVES: &str = "\
from app.repo import Repo


class Service:
    def __init__(self, repo: Repo):
        self.repo = repo

    def show(self, key):
        return self.repo.load(key)

    def list(self, key):
        return self.repo.fetch(key)
";

/// A branch that moves `Repo.load` unchanged into `app/loading.py`, deletes `Repo.fetch`, and
/// adds a `Cache.fetch` of its own; the service's two calls are rewritten.
fn moves_review(tag: &str) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-440-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("app")).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    let write = |name: &str, text: &str| std::fs::write(dir.join(name), text).unwrap();
    write("app/repo.py", LOADS);
    write("app/service.py", SERVES);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    write(
        "app/loading.py",
        "class Loading:\n    def load(self, key):\n        row = self.db.get(key)\n        \
         if row is None:\n            raise KeyError(key)\n        return row\n",
    );
    write(
        "app/repo.py",
        "from app.loading import Loading\n\n\nclass Repo(Loading):\n    pass\n",
    );
    write(
        "app/cache.py",
        "class Cache:\n    def fetch(self, key):\n        return None\n",
    );
    write(
        "app/service.py",
        &SERVES
            .replace("self.repo.load(key)", "self.repo.load(key) or {}")
            .replace("self.repo.fetch(key)", "self.repo.db.fetch(key)"),
    );
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "work"]);
    let review = git::Review::open(&dir, None, None).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir.clone(), Tree::default(), files, Buffer::empty(), None);
    a.start_review(review);
    use_roots(&mut a, Kind::Python, &[]);
    (dir, a)
}

#[test]
fn d_on_a_deleted_call_follows_a_move_and_skips_a_namesake() {
    let (dir, mut a) = moves_review("d-moves");
    let service = dir.join("app/service.py");
    on_deleted(&mut a, &service, "self.repo.load", "load");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at()),
        (Some(&*dir.join("app/loading.py")), File(1)),
        "a method moved unchanged opens on its live copy: {}",
        a.message
    );
    on_deleted(&mut a, &service, "self.repo.fetch", "fetch");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.buf.path.as_deref(),
        Some(&*dir.join("app/repo.py")),
        "a deleted method opens on its red line, not on the branch's namesake: {}",
        a.message
    );
    assert_eq!(a.line_str(), "    def fetch(self, key):");
    assert!(a.deleted.is_some());
    let _ = std::fs::remove_dir_all(dir);
}

/// A review of a branch made on a scratch repository: `base` is committed on `main`, then
/// `branch` writes (`Some`) or deletes (`None`) files on `feature`, and its changes are committed.
fn repo_review(
    tag: &str,
    base: &[(&str, &str)],
    branch: &[(&str, Option<&str>)],
) -> (PathBuf, App) {
    let dir = std::env::temp_dir().join(format!("merl-440-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    let write = |name: &str, text: &str| {
        let p = dir.join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    for (name, text) in base {
        write(name, text);
    }
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    git(&["switch", "-q", "-c", "feature"]);
    for (name, text) in branch {
        match text {
            Some(text) => write(name, text),
            None => std::fs::remove_file(dir.join(name)).unwrap(),
        }
    }
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "work"]);
    let review = git::Review::open(&dir, None, None).unwrap();
    let (_, files) = crate::tree::build(&dir, false);
    let mut a = App::new(dir.clone(), Tree::default(), files, Buffer::empty(), None);
    a.start_review(review);
    use_roots(&mut a, Kind::Python, &[]);
    (dir, a)
}

#[test]
fn d_on_a_deleted_markdown_link_follows_it() {
    let (dir, mut a) = repo_review(
        "d-md",
        &[
            (
                "README.md",
                "# T\n\nSee [guide](docs/guide.md) here.\n\nEnd.\n",
            ),
            ("docs/guide.md", "# Guide\n\ntext\n"),
        ],
        &[("README.md", Some("# T\n\nNothing.\n\nEnd.\n"))],
    );
    on_deleted(&mut a, &dir.join("README.md"), "See", "docs/guide");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.buf.path.as_deref(),
        Some(&*dir.join("docs/guide.md")),
        "follows the link, as on any line: {}",
        a.message
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_rule_that_answers_none_keeps_it_in_a_review() {
    let timer = "class Timer\n{\n    public void Delay(int ms)\n    {\n    }\n}\n";
    let run = "class Run\n{\n    void Go()\n    {\n        Task.Delay(5);\n    }\n}\n";
    let (dir, mut a) = repo_review(
        "d-cs-none",
        &[("Timer.cs", timer), ("Run.cs", run)],
        &[("Timer.cs", Some("class Timer\n{\n}\n"))],
    );
    use_roots(&mut a, Kind::CSharp, &[]);
    a.jump_to(&dir.join("Run.cs"), 5);
    a.col = a.line_str().find("Delay").unwrap();
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.message.as_str()),
        (Some(&*dir.join("Run.cs")), "no definition for Delay"),
        "`Task.Delay` is outside the project, not the `Timer.Delay` the branch deleted"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn d_from_a_branch_line_opens_a_deleted_definition_when_the_branch_has_none() {
    let (dir, mut a) = repo_review(
        "d-fallback",
        &[
            ("lib.py", "def legacy_total(x):\n    return x\n"),
            ("main.py", "def run():\n    return 1\n"),
        ],
        &[
            ("lib.py", Some("X = 1\n")),
            ("main.py", Some("def run():\n    return legacy_total(1)\n")),
        ],
    );
    a.jump_to(&dir.join("main.py"), 2);
    a.col = a.line_str().find("legacy_total").unwrap();
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.buf.path.as_deref(),
        Some(&*dir.join("lib.py")),
        "only the branch's deleted code declares it: {}",
        a.message
    );
    assert_eq!(a.line_str(), "def legacy_total(x):");
    assert!(a.deleted.is_some());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn d_from_a_deleted_line_saves_unsaved_edits_first() {
    let (dir, mut a) = orders_review("d-unsaved");
    let service = dir.join("app/services/orders.py");
    on_deleted(&mut a, &service, "self.total(order_id)", "total");
    let total = a
        .buf
        .lines
        .iter()
        .position(|l| l.contains("def total"))
        .unwrap();
    a.buf.lines.insert(total, String::new());
    a.dirty = true;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.line_str(),
        "    def total(self, order_id):",
        "saved first, on the line the definition is on now: {}",
        a.message
    );
    assert!(!a.dirty);

    on_deleted(
        &mut a,
        &service,
        "get_order_with_items",
        "get_order_with_items",
    );
    let from = a.at();
    (a.dirty, a.conflict) = (true, true);
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at()),
        (Some(&*service), from),
        "edits that cannot be saved: the cursor stays"
    );
    assert!(
        !a.message.contains('\u{2192}'),
        "no resolution: {}",
        a.message
    );
    (a.dirty, a.conflict) = (false, false);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn each_worktree_keeps_its_own_base_and_an_edit_leaves_it_alone() {
    let (dir, a) = repo_review(
        "base-tree",
        &[
            ("svc.py", "X = 1\n"),
            ("keep.py", "def keep():\n    pass\n"),
        ],
        &[("svc.py", Some("X = 2\n"))],
    );
    let base = a.review.as_ref().unwrap().base_tree(&dir).unwrap();
    std::fs::write(dir.join("keep.py"), "def edited():\n    pass\n").unwrap();
    assert_eq!(
        std::fs::read_to_string(base.join("keep.py")).unwrap(),
        "def keep():\n    pass\n",
        "a file the branch left alone is a copy: an edit in place does not reach the base"
    );
    assert_eq!(
        std::fs::read_to_string(base.join("svc.py")).unwrap(),
        "X = 1\n"
    );
    let wt = dir.with_extension("wt");
    let _ = std::fs::remove_dir_all(&wt);
    let git = |at: &Path, args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(at)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            wt.to_str().unwrap(),
            "main",
        ],
    );
    std::fs::write(wt.join("svc.py"), "X = 3\n").unwrap();
    git(&wt, &["commit", "-qam", "base 2"]);
    git(&wt, &["branch", "base2"]);
    std::fs::write(wt.join("svc.py"), "X = 4\n").unwrap();
    git(&wt, &["commit", "-qam", "other"]);
    let other = git::Review::open(&wt, None, Some("base2")).unwrap();
    assert_ne!(
        other.merge_base,
        a.review.as_ref().unwrap().merge_base,
        "another merge base: a commit of its own under the other branch"
    );
    let theirs = other.base_tree(&wt).unwrap();
    assert!(
        base.join("keep.py").exists() && theirs.join("svc.py").exists(),
        "another worktree's review does not remove this one's base"
    );
    let _ = std::fs::remove_dir_all(&wt);
    let _ = std::fs::remove_dir_all(dir);
}

const HELPER: &str = "def helper(x):\n    y = x + 1\n    return y\n";

#[test]
fn d_follows_a_move_into_a_spaced_name_past_the_next_deleted_function() {
    let (dir, mut a) = repo_review(
        "d-move-space",
        &[
            ("svc.py", &format!("{HELPER}def other():\n    return 0\n")),
            (
                "main.py",
                "from svc import helper\n\n\ndef run():\n    helper(1)\n    return 2\n",
            ),
        ],
        &[
            ("svc.py", Some("X = 1\n")),
            ("main.py", Some("def run():\n    return 2\n")),
            ("my mod.py", Some(HELPER)),
        ],
    );
    on_deleted(&mut a, &dir.join("main.py"), "helper(1)", "helper");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at()),
        (Some(&*dir.join("my mod.py")), File(0)),
        "the live copy, past the function deleted under it: {}",
        a.message
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_two_line_block_is_no_move() {
    let short = "def helper(x):\n    return x\n";
    let (dir, mut a) = repo_review(
        "d-move-short",
        &[
            ("svc.py", short),
            (
                "main.py",
                "from svc import helper\n\n\ndef run():\n    helper(1)\n    return 2\n",
            ),
        ],
        &[
            ("svc.py", Some("X = 1\n")),
            ("main.py", Some("def run():\n    return 2\n")),
            ("other.py", Some(short)),
        ],
    );
    on_deleted(&mut a, &dir.join("main.py"), "helper(1)", "helper");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.buf.path.as_deref(),
        Some(&*dir.join("svc.py")),
        "two lines alike are no move: {}",
        a.message
    );
    assert!(a.deleted.is_some());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn d_on_a_deleted_files_line_answers_from_the_base() {
    let (dir, mut a) = moves_review("d-deleted-file");
    let old = "from app.repo import Repo\n\n\ndef use(r: Repo):\n    return r.fetch(1)\n";
    // The branch deletes a file that used `Repo.fetch`: a second commit on top of the review's.
    std::fs::write(dir.join("app/old.py"), old).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    };
    git(&["switch", "-q", "main"]);
    std::fs::write(dir.join("app/old.py"), old).unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "old"]);
    git(&["switch", "-q", "feature"]);
    git(&["rebase", "-q", "main"]);
    git(&["rm", "-q", "app/old.py"]);
    git(&["commit", "-q", "-m", "drop old"]);
    let review = git::Review::open(&dir, None, None).unwrap();
    a.start_review(review);
    a.jump_to(&dir.join("app/old.py"), 5);
    a.col = a.line_str().find("fetch").unwrap();
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.buf.path.as_deref(),
        Some(&*dir.join("app/repo.py")),
        "the base's deleted `Repo.fetch`, not the branch's `Cache.fetch`: {}",
        a.message
    );
    assert_eq!(a.line_str(), "    def fetch(self, key):");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn d_in_a_renamed_file_reads_the_old_name_and_counts_lines_past_the_last_hunk() {
    let lib =
        "import os\nimport sys\n\n\ndef a():\n    return 1\n\n\ndef target():\n    return 2\n";
    let (dir, mut a) = repo_review(
        "d-rename",
        &[
            ("lib.py", lib),
            (
                "run.py",
                "from lib import target\n\n\ndef run():\n    return target()\n",
            ),
        ],
        &[
            ("lib.py", Some(&lib.replace("import sys\n", ""))),
            ("run.py", None),
            (
                "start.py",
                Some("from lib import target\n\n\ndef run():\n    return 0\n"),
            ),
        ],
    );
    let file = a
        .review
        .as_ref()
        .unwrap()
        .file(Path::new("start.py"))
        .cloned();
    assert_eq!(file.and_then(|f| f.old), Some(PathBuf::from("run.py")));
    on_deleted(&mut a, &dir.join("start.py"), "target()", "target");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at()),
        (Some(&*dir.join("lib.py")), File(7)),
        "below the file's last hunk, on its line now: {}",
        a.message
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_base_picker_keeps_its_title_and_marks_its_deleted_row() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::style::Color;
    let (dir, mut a) = repo_review(
        "d-picker",
        &[
            ("a.py", "class A:\n    def run(self):\n        return 1\n"),
            ("b.py", "class B:\n    def run(self):\n        return 2\n"),
            ("main.py", "def go(x):\n    return x.run()\n"),
        ],
        &[
            ("a.py", Some("class A:\n    pass\n")),
            ("main.py", Some("def go(x):\n    return x\n")),
        ],
    );
    on_deleted(&mut a, &dir.join("main.py"), "x.run()", "run");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    let p = a.picker.as_mut().expect("a picker");
    p.settle();
    assert_eq!(p.title, "run: by name, 2 declarations");
    let rows: Vec<(String, bool)> = (p.window(9).0.iter())
        .map(|r| (r.item.label.clone(), r.item.deleted))
        .collect();
    assert!(
        rows.contains(&("A.run  by name  a.py:2: def run(self):".into(), true)),
        "{rows:#?}"
    );
    assert!(
        rows.contains(&("B.run  by name  b.py:2: def run(self):".into(), false)),
        "{rows:#?}"
    );
    a.show_tree = false;
    let theme = crate::theme::load(crate::theme::DEFAULT).unwrap();
    let mut t = Terminal::new(TestBackend::new(90, 12)).unwrap();
    t.draw(|f| crate::ui::draw(f, &mut a, &theme)).unwrap();
    let buf = t.backend().buffer();
    let find = |label: &str| {
        (0..buf.area.height)
            .find_map(|y| {
                let text: String = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
                let at = text.find(label)?;
                Some((text[..at].chars().count() as u16, y))
            })
            .unwrap_or_else(|| panic!("no row {label}"))
    };
    let (xa, ya) = find("A.run");
    let (xb, yb) = find("B.run");
    assert_eq!(
        (buf[(xa - 1, ya)].symbol(), buf[(xa - 1, ya)].fg),
        ("\u{258e}", Color::Red),
        "the deleted row's red mark"
    );
    assert_eq!(buf[(xb - 1, yb)].symbol(), " ");
    let def = |x: u16, y: u16| {
        let row: String = (x..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
        buf[(x + row.find("def").unwrap() as u16, y)].fg
    };
    assert_eq!(
        def(xa, ya),
        def(xb, yb),
        "`def` of the deleted row is coloured as `def` of the other"
    );
    assert_ne!(def(xa, ya), buf[(xa, ya)].fg);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn s_from_a_deleted_line_to_a_live_namesake_misses_no_d() {
    let (dir, mut a) = moves_review("missed-base");
    on_deleted(
        &mut a,
        &dir.join("app/service.py"),
        "self.repo.fetch",
        "fetch",
    );
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "fetch");
    a.settle_search();
    while !a
        .picker
        .as_ref()
        .unwrap()
        .current()
        .unwrap()
        .label
        .starts_with("app/cache.py")
    {
        press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    }
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    a.settle_search();
    assert_eq!(a.buf.path.as_deref(), Some(&*dir.join("app/cache.py")));
    assert!(
        !a.missed.contains_key("d"),
        "`d` would have gone to the deleted `Repo.fetch`: {:?}",
        a.missed
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn d_finds_a_definition_deleted_from_a_component() {
    for (lib, base, now) in [
        (
            "Lib.svelte",
            "<script context=\"module\" lang=\"ts\">\nexport function legacyTotal(x: number) {\n  return x;\n}\n</script>\n<p>hi</p>\n",
            "<script context=\"module\" lang=\"ts\">\n</script>\n<p>hi</p>\n",
        ),
        (
            "lib.ts",
            "// head\nexport function legacyTotal(x: number) {\n  return x;\n}\n// tail\n\n",
            "// head\n// tail\n\n",
        ),
    ] {
        let (dir, mut a) = repo_review(
            &format!("deleted-component-{lib}"),
            &[(lib, base), ("main.ts", "export const a = 1;\n")],
            &[
                (lib, Some(now)),
                ("main.ts", Some("export const a = legacyTotal(1);\n")),
            ],
        );
        use_roots(&mut a, Kind::TsJs, &[]);
        a.jump_to(&dir.join("main.ts"), 1);
        a.col = a.line_str().find("legacyTotal").unwrap();
        press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(a.message, "legacyTotal: by name, 1 match", "{lib}");
        assert!(
            a.deleted.is_some(),
            "{lib}: the red line, whatever the file holds at that number now"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn symbols_of_a_review_skip_a_deleted_template_line() {
    let (dir, mut a) = repo_review(
        "deleted-template",
        &[(
            "Card.vue",
            "<script setup lang=\"ts\">\nfunction save() {}\n</script>\n<template>\nclass Fake {}\n</template>\n",
        )],
        &[(
            "Card.vue",
            Some(
                "<script setup lang=\"ts\">\nfunction save() {}\n</script>\n<template>\n</template>\n",
            ),
        )],
    );
    use_roots(&mut a, Kind::TsJs, &[]);
    a.jump_to(&dir.join("Card.vue"), 1);
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    assert_eq!(
        rows(&mut a),
        ["save  Card.vue:2"],
        "never a line the template lost"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_base_picker_row_of_a_renamed_file_names_the_file_it_has_now() {
    let (dir, mut a) = repo_review(
        "d-picker-renamed",
        &[
            ("a.py", "class A:\n    def run(self):\n        return 1\n"),
            ("b.py", "class B:\n    def run(self):\n        return 2\n"),
            ("main.py", "def go(x):\n    return x.run()\n"),
        ],
        &[
            ("b.py", None),
            (
                "lib/beta.py",
                Some("class B:\n    def run(self):\n        return 2\n"),
            ),
            ("main.py", Some("def go(x):\n    return x\n")),
        ],
    );
    on_deleted(&mut a, &dir.join("main.py"), "x.run()", "run");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    let p = a.picker.as_mut().expect("a picker");
    p.settle();
    let paths: Vec<String> = (p.window(9).0.iter())
        .map(|r| r.item.label[r.item.path_at.clone().unwrap()].to_string())
        .collect();
    assert_eq!(paths, ["a.py", "lib/beta.py"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn symbols_of_a_review_list_no_deleted_line_of_a_file_no_row_reads() {
    let (dir, mut a) = repo_review(
        "deleted-notes",
        &[
            ("a.py", "def kept():\n    pass\n"),
            ("notes.md", "def gone():\nx\n"),
        ],
        &[("notes.md", Some("x\n"))],
    );
    a.jump_to(&dir.join("a.py"), 1);
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    assert_eq!(rows(&mut a), ["kept  a.py:1"]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn deleted_lines_joining_a_search_are_cut_where_its_grep_is() {
    let (dir, mut a) = repo_review(
        "deleted-cap",
        &[("a.txt", &"x\n".repeat(search::MAX_HITS + 10))],
        &[("a.txt", Some(&"x\n".repeat(search::MAX_HITS - 10)))],
    );
    a.jump_to(&dir.join("a.txt"), 1);
    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "x");
    a.settle_search();
    let p = a.picker.as_mut().unwrap();
    p.settle();
    assert_eq!(p.counts().1 as usize, search::MAX_HITS);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_selection_from_the_line_under_a_deletion_is_edited_and_one_over_it_is_not() {
    let (dir, mut a) = repo_review(
        "select-deleted",
        &[("a.txt", "a\nb\nc\nd\n")],
        &[("a.txt", Some("a\nc\nd\n"))],
    );
    let file = dir.join("a.txt");
    a.jump_to(&file, 2);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    typed(&mut a, "x");
    assert_eq!(a.buf.lines, ["a", "xd"], "{}", a.message);
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    a.jump_to(&file, 1);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    press(&mut a, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(
        (a.buf.lines.join(","), a.message.as_str()),
        ("a,c,d".into(), "deleted")
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_reload_carries_a_selection_anchored_on_a_deleted_line() {
    let (dir, mut a) = repo_review(
        "anchor-deleted",
        &[("a.txt", "a\nb\nc\nd\n")],
        &[("a.txt", Some("a\nc\nd\n"))],
    );
    let file = dir.join("a.txt");
    a.jump_to(&file, 2);
    press(&mut a, KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(a.at(), Deleted(1, 0));
    press(&mut a, KeyCode::Down, KeyModifiers::SHIFT);
    std::fs::write(&file, "x\ny\na\nc\nd\n").unwrap();
    a.reload(false);
    assert_eq!(a.selected_text().as_deref(), Some("b\n"));
    let _ = std::fs::remove_dir_all(dir);
}
