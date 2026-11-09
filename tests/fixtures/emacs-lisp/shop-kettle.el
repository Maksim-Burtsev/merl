;;; shop-kettle.el --- a definition with its name under the head (#1431)  -*- lexical-binding: t -*-
(defun
    shop-kettle-boil (x) (* 2 x))
(defun
    ;; shop-kettle-boil
    shop-kettle-cool (x) x)
(defun shop-kettle-use ()
  (shop-kettle-boil 1)
;  ^ d: shop-kettle.el:2
  (shop-kettle-cool 2))
;  ^ d: shop-kettle.el:4
(defun shop-kettle-note ()
  "Boil with `shop-kettle-boil' first."
;             ^ d: shop-kettle.el:2
  ;; then `shop-kettle-cool'
;          ^ d: shop-kettle.el:4
  nil)
