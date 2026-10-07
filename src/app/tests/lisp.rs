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
        ["src/shop/money.clj:2:", "src/shop/cart.clj:2:"]
    );
    assert_eq!(
        rows(&mut a, 2, "price 1"),
        ["src/shop/money.clj:4:", "src/shop/cart.clj:2:"]
    );
    assert_eq!(rows(&mut a, 3, "empty?"), ["src/shop/cart.clj:3:"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d_on_a_require_the_project_lacks_opens_it_in_the_roots() {
    let (dir, mut a) = project_app(
        "lisp-roots",
        &[
            ("init.el", "(require 'magit)\n(require 'missing)\n"),
            ("main.rkt", "#lang racket\n(require racket/list)\n"),
            ("lib.scm", "(define (second-of l) (rest l))\n"),
            ("racket.ss", "#lang racket\n(define (tail-of l) (rest l))\n"),
            (
                "bare.rkt",
                "(module bare racket\n  (define (head-of l) (rest l)))\n",
            ),
        ],
    );
    let root = external_root(
        "lisp",
        &[
            ("elpa/magit-4.1/magit.el", ";;; magit.el\n"),
            (
                "collects/racket/list.rkt",
                "#lang racket/base\n(define (rest l) (cdr l))\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::EmacsLisp, &[root.join("elpa")]);
    use_roots(&mut a, Kind::Scheme, &[root.join("collects")]);
    let at = |p: &str| format!("{}", root.join(p).display());
    for (file, code, want) in [
        (
            "init.el",
            "magit",
            jump(
                &format!("magit.el: module {}", at("elpa/magit-4.1/magit.el")),
                &at("elpa/magit-4.1/magit.el:1"),
            ),
        ),
        (
            "init.el",
            "missing",
            jump("no definition for missing.el", "init.el:2"),
        ),
        (
            "main.rkt",
            "racket/list",
            jump(
                &format!("racket/list.rkt: module {}", at("collects/racket/list.rkt")),
                &at("collects/racket/list.rkt:1"),
            ),
        ),
        (
            "lib.scm",
            "rest",
            jump("no definition for rest", "lib.scm:1"),
        ),
        (
            "racket.ss",
            "rest",
            jump("rest: by name, 1 match", &at("collects/racket/list.rkt:2")),
        ),
        (
            "bare.rkt",
            "rest",
            jump("rest: by name, 1 match", &at("collects/racket/list.rkt:2")),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}
