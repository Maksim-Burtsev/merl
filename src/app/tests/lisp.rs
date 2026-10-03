use super::*;

#[test]
fn u_reads_a_lisp_name_whole_with_its_marks() {
    let (dir, mut a) = project_app(
        "lisp-u",
        &[
            (
                "src/shop/money.clj",
                "(ns shop.money)\n(defn format-price [c] c)\n(defn format [c] c)\n(defn price [c] c)\n",
            ),
            (
                "src/shop/cart.clj",
                "(ns shop.cart (:require [shop.money :as m]))\n(m/format-price (m/format (m/price 1)))\n(empty? (empty []))\n",
            ),
        ],
    );
    let rows = |a: &mut App, line: usize, at: &str| {
        a.jump_to(&dir.join("src/shop/cart.clj"), line);
        a.col = a.line_str().find(at).expect(at);
        press(a, KeyCode::Char('u'), KeyModifiers::NONE);
        let picker = a.picker.as_mut().expect("a picker");
        picker.settle();
        let rows: Vec<String> = (picker.window(50).0.into_iter())
            .map(|r| {
                r.item.label[..r.item.code_at.unwrap()]
                    .trim_end()
                    .to_owned()
            })
            .collect();
        press(a, KeyCode::Esc, KeyModifiers::NONE);
        rows
    };
    assert_eq!(
        rows(&mut a, 2, "format-price"),
        [
            "declaration  src/shop/money.clj:2:",
            "             src/shop/cart.clj:2:"
        ]
    );
    assert_eq!(
        rows(&mut a, 2, "price 1"),
        [
            "declaration  src/shop/money.clj:4:",
            "             src/shop/cart.clj:2:"
        ]
    );
    assert_eq!(rows(&mut a, 3, "empty?"), ["src/shop/cart.clj:3:"]);
    std::fs::remove_dir_all(&dir).unwrap();
}
