use super::*;

fn declares(kind: Kind, line: &str, word: &str) -> bool {
    Regex::new(&lisp_patterns(kind, word).join("|"))
        .unwrap()
        .is_match(line)
}

#[test]
fn lisp_declaration_forms() {
    for (kind, line, word) in [
        (Kind::Clojure, "(defn format-price", "format-price"),
        (Kind::Clojure, "(defn- ^:private ^String round [x]", "round"),
        (Kind::Clojure, "(def ^{:doc \"x\"} rate 1)", "rate"),
        (Kind::Clojure, "(defrecord Point [x y])", "->Point"),
        (Kind::Clojure, "(defrecord Point [x y])", "map->Point"),
        (Kind::Clojure, "  (area [this])", "area"),
        (
            Kind::EmacsLisp,
            "(defalias 'shop-total #'shop-money-total)",
            "shop-total",
        ),
        (
            Kind::EmacsLisp,
            "(cl-defstruct (shop-order (:constructor x)) id)",
            "shop-order",
        ),
        (
            Kind::EmacsLisp,
            "(transient-define-prefix shop-menu ()",
            "shop-menu",
        ),
        (Kind::Scheme, "(define (parse s)", "parse"),
        (
            Kind::Scheme,
            "(define-values (low high) (values 1 9))",
            "high",
        ),
        (Kind::Scheme, "(struct point (x y))", "point"),
        (Kind::CommonLisp, "(DEFUN Format-Price (x)", "format-price"),
        (Kind::CommonLisp, "(defpackage #:shop", "shop"),
        (Kind::CommonLisp, "(defsystem \"shop\"", "shop"),
        (
            Kind::CommonLisp,
            "(defstruct (point (:constructor make-pt)) x)",
            "point",
        ),
    ] {
        assert!(declares(kind, line, word), "{line}: {word}");
    }
    for (kind, line, word) in [
        (Kind::Clojure, "(format-price 1)", "format-price"),
        (Kind::Clojure, "(s/def ::order map?)", "order"),
        (Kind::Clojure, "#_(defn ignored [] 1)", "ignored"),
        (Kind::Clojure, "(defn empty-cart? [c])", "empty"),
        (
            Kind::EmacsLisp,
            "(defadvice shop-label (around x))",
            "shop-label",
        ),
        (Kind::EmacsLisp, "(require 'shop-money)", "shop-money"),
        (Kind::CommonLisp, "(in-package :shop)", "shop"),
    ] {
        assert!(!declares(kind, line, word), "{line}: {word}");
    }
}

#[test]
fn lisp_literals_run_over_lines() {
    let lit = |kind, text: &str| -> Vec<usize> {
        literal_lines(kind, text)
            .iter()
            .enumerate()
            .filter(|(_, l)| **l)
            .map(|(i, _)| i + 1)
            .collect()
    };
    let clj = "(defn f\n  \"doc\n  (defn g [])\"\n  [x] \\\" x)\n(comment\n  (defn h []))\n#_\n(defn i [])\n(defn j [] `(a))\n";
    assert_eq!(lit(Kind::Clojure, clj), [3, 6, 8]);
    let el = "(defun f () ?\\\" (g))\n#| x\n(defun h ())\n";
    assert_eq!(lit(Kind::EmacsLisp, el), Vec::<usize>::new());
    let scm = "#| a\n#| b |#\n(define (h) 1)\n|#\n(define c #\\\")\n#;(define\n  (i) 2)\n(define (j) 3)\n";
    assert_eq!(lit(Kind::Scheme, scm), [2, 3, 4, 7]);
    let cl = "(defun f ()\n  \"a; b\n(defun g ())\" ; \"\n  1)\n";
    assert_eq!(lit(Kind::CommonLisp, cl), [3]);
}

#[test]
fn lisp_names_keep_their_marks() {
    let name = |kind, line: &str, at: &str| {
        let col = line.find(at).unwrap();
        definition_word(Some(kind), line, col).map(|(_, w)| w.to_owned())
    };
    let clj = "(empty? (->Point *out*) money/format-price)";
    assert_eq!(name(Kind::Clojure, clj, "empty").as_deref(), Some("empty?"));
    assert_eq!(
        name(Kind::Clojure, clj, "Point").as_deref(),
        Some("->Point")
    );
    assert_eq!(name(Kind::Clojure, clj, "out").as_deref(), Some("*out*"));
    assert_eq!(
        name(Kind::Clojure, clj, "price").as_deref(),
        Some("format-price")
    );
    assert_eq!(name(Kind::Clojure, clj, "money").as_deref(), Some("money"));
    let el = "(doom/reload :key)";
    assert_eq!(
        name(Kind::EmacsLisp, el, "reload").as_deref(),
        Some("doom/reload")
    );
    assert_eq!(name(Kind::EmacsLisp, el, "key").as_deref(), Some("key"));
    let cl = "(alexandria:when-let %x)";
    assert_eq!(
        name(Kind::CommonLisp, cl, "when").as_deref(),
        Some("when-let")
    );
    assert_eq!(
        name(Kind::Scheme, "(racket/list)", "list").as_deref(),
        Some("racket/list")
    );
}

#[test]
fn lisp_symbols() {
    for (kind, line, name) in [
        (
            Kind::Clojure,
            "(defn ^:private format-price [x]",
            "format-price",
        ),
        (Kind::Clojure, "(ns shop.money", "shop.money"),
        (
            Kind::EmacsLisp,
            "(cl-defstruct (shop-order (:constructor x))",
            "shop-order",
        ),
        (Kind::EmacsLisp, "(define-minor-mode shop-mode", "shop-mode"),
        (Kind::Scheme, "(define (parse s)", "parse"),
        (Kind::Scheme, "(define-syntax-rule (swap! a b)", "swap!"),
        (Kind::CommonLisp, "(defpackage #:shop", "shop"),
        (Kind::CommonLisp, "(DEFUN format-price (x)", "format-price"),
    ] {
        assert_eq!(one(kind, line).as_deref(), Some(name), "{line}");
    }
    for (kind, line) in [
        (Kind::Clojure, "(def rate 1)"),
        (Kind::Clojure, "(defmethod price :circle [s])"),
        (Kind::EmacsLisp, "(defvar shop-items nil)"),
        (Kind::EmacsLisp, "(cl-defmethod shop-price ((x y)))"),
        (Kind::Scheme, "(define tax 0.2)"),
        (Kind::CommonLisp, "(defparameter *x* 1)"),
        (Kind::CommonLisp, "(defmethod price ((x integer)))"),
    ] {
        assert!(listed(kind, line).is_empty(), "{line}");
    }
}

#[test]
fn clojure_requires_bind_aliases_and_referred_names() {
    let text = "(ns shop.cart\n  (:require [shop.money :as money :refer [format-price]]\n            shop.util))\n(require '[shop.tax :as tax])\n";
    let got = clojure_requires(text);
    for pair in [
        ("money", "shop.money"),
        ("format-price", "shop.money"),
        ("shop.util", "shop.util"),
        ("tax", "shop.tax"),
    ] {
        assert!(
            got.contains(&(pair.0.into(), pair.1.into())),
            "{pair:?} in {got:?}"
        );
    }
    let files = [
        PathBuf::from("src/shop/money_util.cljc"),
        PathBuf::from("money.clj"),
    ];
    assert_eq!(
        clojure_ns_files("shop.money-util", &files),
        [files[0].clone()]
    );
}

#[test]
fn lisp_locals_stay_in_their_form() {
    let at = |kind, text: &str, line, name| -> Vec<usize> {
        bindings(kind, text, line, name)
            .iter()
            .map(|b| b.line)
            .collect()
    };
    let clj =
        "(defn f [{:keys [a]} b]\n  (let [x 1\n        [y z] [2 3]]\n    (+ a b x y z))\n  x)\n";
    for (name, line) in [("a", 1), ("b", 1), ("x", 2), ("y", 3), ("z", 3)] {
        assert_eq!(at(Kind::Clojure, clj, 4, name), [line], "{name}");
    }
    assert!(at(Kind::Clojure, clj, 5, "x").is_empty());
    let el = "(defun f (cart &optional sep)\n  (let ((total 1) count)\n    (list cart sep total count)))\n";
    for (name, line) in [("cart", 1), ("sep", 1), ("total", 2), ("count", 2)] {
        assert_eq!(at(Kind::EmacsLisp, el, 3, name), [line], "{name}");
    }
    let scm = "(define (f a)\n  (let loop ([i 0])\n    (lambda (b) (+ a b))))\n";
    assert_eq!(at(Kind::Scheme, scm, 3, "a"), [1]);
    assert_eq!(at(Kind::Scheme, scm, 3, "b"), [3]);
    assert!(at(Kind::Scheme, scm, 3, "f").is_empty());
    let cl = "(defun f (X)\n  (labels ((g (y) y))\n    (g x)))\n";
    assert_eq!(at(Kind::CommonLisp, cl, 3, "x"), [1]);
    assert_eq!(at(Kind::CommonLisp, cl, 3, "g"), [2]);
}
