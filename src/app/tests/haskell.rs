use super::*;

fn rows(a: &mut App) -> Vec<String> {
    let picker = a.picker.as_mut().expect("a picker");
    picker.settle();
    let rows = (picker.window(50).0.into_iter())
        .map(|r| {
            r.item
                .label
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    press(a, KeyCode::Esc, KeyModifiers::NONE);
    rows
}

#[test]
fn u_reads_a_primed_haskell_name_whole() {
    let (dir, mut a) = project_app(
        "haskell-u",
        &[(
            "src/Shop.hs",
            "total :: [Int] -> Int\ntotal = sum\n\ntotal' :: [Int] -> Int\ntotal' = foldr (+) 0\n\nboth xs = total xs + total' xs\n",
        )],
    );
    a.jump_to(&dir.join("src/Shop.hs"), 7);
    a.col = a.line_str().find("total'").unwrap();
    press(&mut a, KeyCode::Char('u'), KeyModifiers::NONE);
    let found = rows(&mut a);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found[0].starts_with("declaration src/Shop.hs:4:"),
        "{found:?}"
    );
    assert!(found.iter().all(|r| r.contains("total'")), "{found:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_lists_haskell_declarations_once() {
    let (dir, mut a) = project_app(
        "haskell-symbols",
        &[(
            "src/Shop.hs",
            "module Shop where\n\nimport Data.List\n\ndata Coupon = Flat Int\n\nrate :: Coupon -> Int\nrate (Flat n) = n\n\nband 0 = 1\nband n = 2\n\ninstance Show Coupon where\n  show c = \"coupon\"\n\n{-\nghost = 1\n-}\n",
        )],
    );
    a.jump_to(&dir.join("src/Shop.hs"), 1);
    press(&mut a, KeyCode::Char('D'), KeyModifiers::NONE);
    let mut names: Vec<String> = rows(&mut a)
        .iter()
        .map(|r| r.split(' ').next().unwrap().to_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["Coupon", "band", "rate"]);
    std::fs::remove_dir_all(&dir).unwrap();
}
