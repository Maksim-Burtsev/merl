(in-package :shop)

(defun label (cart &optional (sep " "))
  (let* ((total (reduce #'+ cart))
;                             ^ d: shop.lisp:3
; status: local
         (quote-char #\"))
    (flet ((double (x) (* 2 x)))
      (concatenate 'string (shop-money:Format-Price (double total)) sep))))
;                                            ^ d: picker money.lisp:7, money.lisp:23
;                                                       ^ d: shop.lisp:8
; status: local
;                                                             ^ d: shop.lisp:4
;                                                                     ^ d: shop.lisp:3

(defun run ()
  (shop-money::with-cents (x) (price x)) (make-instance 'order) (make-pt) (make-parcel)
;                   ^ d: money.lisp:15
;                                 ^ d: picker money.lisp:16, money.lisp:17
;                                                           ^ d: money.lisp:18
;                                                                   ^ d: none
  (error 'shop-error) (typep 1 'cents) (list +tax+ (hidden) (still-hidden) (fake-price) (point-x 1)))
;              ^ d: money.lisp:22
;                                 ^ d: money.lisp:21
;                                              ^ d: money.lisp:6
;                                                      ^ d: none
;                                                                  ^ d: none
;                                                                                ^ d: none
;                                                                                           ^ d: none
