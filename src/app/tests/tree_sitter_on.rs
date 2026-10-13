use super::*;
use ratatui::{Terminal, backend::TestBackend};

use crate::buffer::{Spans, use_tree_sitter};

struct Switch;

impl Switch {
    fn on() -> Self {
        use_tree_sitter(true);
        Self
    }
}

impl Drop for Switch {
    fn drop(&mut self) {
        use_tree_sitter(false);
    }
}

fn settle(a: &mut App) {
    let mut steps = 0;
    while a.parsing() {
        assert!(a.parse_more() || a.parsing());
        steps += 1;
        assert!(steps < 100_000, "parse_more never finishes");
    }
}

fn painted(b: &mut Buffer, theme: &crate::theme::Theme) -> Vec<Spans> {
    assert!(!b.sexp().is_empty(), "no tree for {:?}", b.path);
    b.paint_to(usize::MAX, theme);
    b.hl.clone()
}

fn fresh(path: &Path, bytes: &[u8], theme: &crate::theme::Theme) -> Vec<Spans> {
    let mut b = Buffer::from_bytes(path.to_path_buf(), bytes);
    b.parse_more(None);
    painted(&mut b, theme)
}

#[test]
fn the_setting_reaches_every_buffer_the_app_opens() {
    let _on = Switch::on();
    let theme = crate::theme::load("tokyonight-moon").unwrap();
    let base = "def f():\n    return 1\n\n\nx = f()\n";
    let work = "def f():\n    return 2\n\n\nx = f()\n";
    let (dir, _) = review_app_with("ts-on", &[]);
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
    std::fs::write(dir.join("a.py"), base).unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "a.py"]);
    git(&["switch", "-q", "feature"]);
    git(&["merge", "-q", "main"]);
    std::fs::write(dir.join("a.py"), work).unwrap();
    git(&["commit", "-q", "-am", "a.py work"]);
    let mut a = review_start(&dir, None);
    let path = dir.join("a.py");
    a.jump_to(&path, 0);
    settle(&mut a);
    assert_eq!(
        painted(&mut a.buf, &theme),
        fresh(&path, work.as_bytes(), &theme)
    );
    let (_, b) = a.base.as_mut().expect("the base of a.py, for its ghost");
    assert_eq!(painted(b, &theme), fresh(&path, base.as_bytes(), &theme));

    let changed = "def g():\n    return 3\n\n\nx = g()\n".to_owned()
        + &"z = {\"k\": [1, 2, (3, 4)]}  # c\n".repeat(5000);
    let changed = changed.as_str();
    std::fs::write(&path, changed).unwrap();
    assert!(a.reload(false));
    settle(&mut a);
    assert_eq!(
        painted(&mut a.buf, &theme),
        fresh(&path, changed.as_bytes(), &theme)
    );

    press(&mut a, KeyCode::Char('s'), KeyModifiers::NONE);
    typed(&mut a, "return");
    a.settle_search();
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|f| crate::ui::draw(f, &mut a, &theme))
        .unwrap();
    let picker = a.picker.as_ref().expect("the search picker");
    assert!(picker.bufs[&PathBuf::from("a.py")].parsing());
    settle(&mut a);
    let picker = a.picker.as_mut().expect("the search picker");
    let b = picker
        .bufs
        .get_mut(&PathBuf::from("a.py"))
        .expect("a.py's preview");
    assert_eq!(painted(b, &theme), fresh(&path, changed.as_bytes(), &theme));
    drop(std::fs::remove_dir_all(&dir));
}

#[test]
fn the_setting_reaches_the_code_blocks_of_the_preview() {
    let _on = Switch::on();
    let theme = crate::theme::load("tokyonight-moon").unwrap();
    let dir = std::env::temp_dir().join(format!("merl-ts-on-md-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("PLAN.md");
    std::fs::write(&path, "# Plan\n\n```rs\nfn main() {}\n```\n").unwrap();
    let mut a = App::new(
        dir.clone(),
        Tree::default(),
        Vec::new(),
        Buffer::load(&path).unwrap(),
        None,
    );
    (a.view_w, a.view_h) = (40, 8);
    press(&mut a, KeyCode::Char('p'), KeyModifiers::NONE);
    settle(&mut a);
    let code = &mut a.preview.as_mut().expect("the preview").code[0];
    let rust = fresh(Path::new("main.rs"), b"fn main() {}\n", &theme);
    assert_eq!(painted(code, &theme)[0], rust[0]);
    drop(std::fs::remove_dir_all(&dir));
}
