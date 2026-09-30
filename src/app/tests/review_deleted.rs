//! #440: in a review, `s`, `D`, `u` and `d` find the code the branch deleted.

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

/// The cursor on `word` of the deleted line of `file` that holds `line`.
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

/// `d` on the deleted call lands on the deleted definition in another file, red, and `[` goes
/// back to the call. The base proves it: `self.repo` is the `OrderRepository` its `__init__` took.
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
    assert_eq!(a.at(), Deleted(3, 1), "{}", a.message);
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
    assert_eq!((a.buf.path.as_deref(), a.at()), (Some(&*service), from));
    let _ = std::fs::remove_dir_all(dir);
}

/// From a file line, a name the branch still defines goes to the branch's definition, though
/// the deleted `app/legacy.py` declares a `get` too.
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
        "{}",
        a.message
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// From a deleted line, `d` answers as the base had the code: a definition the branch kept on
/// its line now, one it rewrote elsewhere on its old, deleted text.
#[test]
fn d_on_a_deleted_line_answers_from_the_base() {
    let (dir, mut a) = orders_review("d-base");
    let repo = dir.join("app/repositories/orders.py");
    // `self.get` on the repository's deleted line.
    on_deleted(&mut a, &repo, "order = self.get", "get(");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.at(), a.picker.is_none()),
        (File(1), true),
        "{}",
        a.message
    );
    // The call of `fmt` is deleted with it, and the branch's `fmt` in `app/format.py` says
    // `repr`: the answer is the `fmt` the call called.
    let service = dir.join("app/services/orders.py");
    on_deleted(&mut a, &service, "[fmt(o)", "fmt");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.line_str()),
        (Some(&*service), "def fmt(order):"),
        "{}",
        a.message
    );
    assert!(a.deleted.is_some());
    // `self.total` on a deleted line: the method under the block, on its line of the file.
    on_deleted(&mut a, &service, "self.total", "total");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at(), a.line_str()),
        (Some(&*service), File(10), "    def total(self, order_id):"),
        "{}",
        a.message
    );
    // `order` is bound by the deleted line above it, in the block deleted with it.
    on_deleted(&mut a, &repo, "return order", "order");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(a.at(), Deleted(3, 2), "{}", a.message);
    let _ = std::fs::remove_dir_all(dir);
}

/// `s` lists the deleted lines numbered as the base had them, among their file's, and Enter lands
/// on one.
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
        ]
    );
    assert!(a.picker.as_ref().unwrap().current().unwrap().deleted);
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at()),
        (
            Some(&*dir.join("app/repositories/orders.py")),
            Deleted(3, 1)
        )
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// The cursor of `s` stays on its row as the query grows: `app/services/orders.py:9` is not
/// the deleted line 9 drawn above it.
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
    assert!(cur.label.starts_with(row) && !cur.deleted, "{}", cur.label);
    let _ = std::fs::remove_dir_all(dir);
}

/// `u` lists the deleted uses beside the branch's, a deleted declaration marked as one; `D` lists
/// the deleted declarations.
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
    assert!(
        listed
            .iter()
            .any(|r| r.starts_with("declaration") && r.contains("app/legacy.py:1: def get(key):")),
        "{listed:#?}"
    );
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);

    // From the deleted call, its uses: both deleted.
    on_deleted(
        &mut a,
        &dir.join("app/services/orders.py"),
        "get_order_with_items",
        "get_order_with_items",
    );
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    assert_eq!(rows(&mut a).len(), 2, "{}", a.message);
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

/// A deleted row of `s` differs from the others only by its mark: the gutter's `▎`, red, and the
/// row is drawn as any other, selected here.
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
    let label = "app/services/orders.py:9: self.total(order_id)";
    let (x, y) = (0..buf.area.height)
        .find_map(|y| {
            let text: String = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
            let at = text.find(label)?;
            Some((text[..at].chars().count() as u16, y))
        })
        .expect("the deleted row");
    assert_eq!(
        (buf[(x - 1, y)].symbol(), buf[(x - 1, y)].fg),
        ("\u{258e}", Color::Red)
    );
    assert_eq!(buf[(x, y)].bg, theme.line_hl, "selected");
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

/// From a deleted call, a method the branch moved unchanged opens on its live copy, and a method
/// it deleted opens on its red line though the branch declares a namesake elsewhere.
#[test]
fn d_on_a_deleted_call_follows_a_move_and_skips_a_namesake() {
    let (dir, mut a) = moves_review("d-moves");
    let service = dir.join("app/service.py");
    on_deleted(&mut a, &service, "self.repo.load", "load");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        (a.buf.path.as_deref(), a.at()),
        (Some(&*dir.join("app/loading.py")), File(1)),
        "{}",
        a.message
    );
    on_deleted(&mut a, &service, "self.repo.fetch", "fetch");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.buf.path.as_deref(),
        Some(&*dir.join("app/repo.py")),
        "{}",
        a.message
    );
    assert_eq!(a.line_str(), "    def fetch(self, key):");
    assert!(a.deleted.is_some());
    let _ = std::fs::remove_dir_all(dir);
}
