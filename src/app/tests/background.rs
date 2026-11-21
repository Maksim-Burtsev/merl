use super::review_deleted::{on_deleted, orders_review};
use super::*;

const SEARCHING: &str = "searching…";

fn slow_project(tag: &str) -> (PathBuf, App) {
    let (dir, mut a) = project_app(
        &format!("background-{tag}"),
        &[
            (
                "a.py",
                "import b\n\nvalue = b.make()\nvalue.frobnicate()\nlocal = 1\nprint(local)\n",
            ),
            (
                "b.py",
                "class Maker:\n    def frobnicate(self):\n        pass\n",
            ),
        ],
    );
    let read_blocks_until_released = dir.join("slow.py");
    let made = std::process::Command::new("mkfifo")
        .arg(&read_blocks_until_released)
        .status();
    assert!(made.unwrap().success());
    a.files.push("slow.py".into());
    a.no_external();
    a.background.waits = Some(Duration::from_millis(20));
    (dir, a)
}

fn release(dir: &Path) {
    let pipe = dir.join("slow.py");
    std::thread::spawn(move || {
        while let Ok(mut reader_came) = std::fs::OpenOptions::new().write(true).open(&pipe) {
            drop(std::io::Write::write_all(&mut reader_came, b"\n"));
        }
    });
}

fn tick_until(a: &mut App, done: impl Fn(&App) -> bool) {
    let gives_up = Instant::now() + Duration::from_secs(30);
    while !done(a) {
        assert!(Instant::now() < gives_up, "still {}", a.message);
        std::thread::sleep(Duration::from_millis(2));
        a.tick();
    }
}

fn searching_from(tag: &str) -> (PathBuf, App, (PathBuf, usize)) {
    let (dir, mut a) = slow_project(tag);
    d_on(&mut a, "a.py", "value.frobnicate");
    assert_eq!(a.message, SEARCHING);
    assert!(a.searching());
    let asked_at = at(&a);
    (dir, a, asked_at)
}

#[test]
fn a_d_that_outlasts_the_wait_says_searching_and_lands_when_answered() {
    let (dir, mut a, asked_at) = searching_from("lands");
    a.tick();
    assert_eq!((at(&a), a.message.as_str()), (asked_at, SEARCHING));
    release(&dir);
    tick_until(&mut a, |a| !a.searching());
    assert_eq!(at(&a), (dir.join("b.py"), 1));
    assert_eq!(
        a.message,
        "frobnicate → Maker.frobnicate (by name, 1 match)"
    );
    assert_eq!(a.pressed.get("d"), Some(&1));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_d_answered_within_the_wait_lands_at_once() {
    let (dir, mut a) = slow_project("at-once");
    a.background.waits = Some(Duration::MAX);
    d_on(&mut a, "a.py", "print(local");
    assert_eq!(at(&a), (dir.join("a.py"), 4));
    assert_eq!(a.message, "local: local");
    assert!(!a.searching());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn esc_drops_the_search() {
    let (dir, mut a, asked_at) = searching_from("esc");
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    assert!(!a.searching());
    assert_eq!(a.message, "");
    release(&dir);
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(2));
        a.tick();
    }
    assert_eq!((at(&a), a.message.as_str()), (asked_at, ""));
    assert_eq!(a.pressed.get("d"), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_key_that_moves_the_cursor_drops_the_search() {
    let (dir, mut a, asked_at) = searching_from("moves");
    press(&mut a, KeyCode::Down, KeyModifiers::NONE);
    a.tick();
    assert!(!a.searching());
    assert_eq!(
        (at(&a), a.message.as_str()),
        ((asked_at.0, asked_at.1 + 1), "")
    );
    release(&dir);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_key_that_moves_nothing_leaves_the_search_running() {
    let (dir, mut a, asked_at) = searching_from("stays");
    press(&mut a, KeyCode::Char('t'), KeyModifiers::NONE);
    assert!(!a.show_tree);
    a.tick();
    assert_eq!((at(&a), a.message.as_str()), (asked_at, SEARCHING));
    release(&dir);
    tick_until(&mut a, |a| !a.searching());
    assert_eq!(at(&a), (dir.join("b.py"), 1));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_second_d_drops_the_first_search_and_starts_its_own() {
    let (dir, mut a, _) = searching_from("second");
    d_on(&mut a, "a.py", "print(local");
    tick_until(&mut a, |a| !a.searching());
    assert_eq!(at(&a), (dir.join("a.py"), 4));
    assert_eq!(a.message, "local: local");
    release(&dir);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_answer_waits_while_a_prompt_is_open() {
    let (dir, mut a, asked_at) = searching_from("prompt");
    press(&mut a, KeyCode::Char(':'), KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Goto);
    release(&dir);
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(2));
        a.tick();
        assert_eq!((at(&a), a.mode), (asked_at.clone(), Mode::Goto));
    }
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(a.mode, Mode::Normal);
    tick_until(&mut a, |a| !a.searching());
    assert_eq!(at(&a), (dir.join("b.py"), 1));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_dropped_search_stops_at_the_next_file_it_greps() {
    let (dir, a) = slow_project("stops");
    let dropped = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let (root, files) = (a.root, vec![PathBuf::from("b.py")]);
    let stopped = std::thread::spawn(move || {
        search::drop_this_thread_s_search_on(dropped);
        search::grep_project(&root, &files, "frobnicate", false, false, None, None).map(drop)
    })
    .join();
    assert!(stopped.is_err_and(|why| why.is::<search::Dropped>()));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_in_a_review_answers_in_the_background_as_on_the_spot() {
    let seen = |tag: &str, waits: Option<Duration>| {
        let (dir, mut a) = orders_review(tag);
        a.background.waits = waits;
        let service = dir.join("app/services/orders.py");
        let repo = dir.join("app/repositories/orders.py");
        let mut all = Vec::new();
        let mut d = |a: &mut App| {
            press(a, KeyCode::Char('d'), KeyModifiers::NONE);
            let picked = a.picker.as_mut().map(|p| {
                p.settle();
                let rows = p.window(usize::MAX).0;
                let rows: Vec<String> = rows.into_iter().map(|r| r.item.label).collect();
                (p.title.clone(), rows)
            });
            let now = format!(
                "{:?}",
                (a.pos(), a.line_str(), &a.message, picked, &a.history)
            );
            all.push(now.replace(&*dir.to_string_lossy(), ""));
            press(a, KeyCode::Esc, KeyModifiers::NONE);
        };
        for (file, line, word) in [
            (&service, "get_order_with_items", "get_order_with_items"),
            (&repo, "order = self.get", "get("),
            (&service, "[fmt(o)", "fmt"),
            (&service, "self.total", "total"),
            (&repo, "return order", "order"),
        ] {
            on_deleted(&mut a, file, line, word);
            d(&mut a);
        }
        a.jump_to(&service, 12);
        a.col = a.line_str().rfind("get").unwrap();
        d(&mut a);
        drop(std::fs::remove_dir_all(dir));
        all
    };
    let on_the_spot = seen("background-spot", None);
    assert_eq!(on_the_spot.len(), 6);
    assert!(on_the_spot[0].contains("Deleted(3, 1)"), "{on_the_spot:?}");
    assert_eq!(on_the_spot, seen("background-review", Some(Duration::MAX)));
}
