;;; shop.el --- the shop (#428)  -*- lexical-binding: t -*-
(require 'shop-money)
;              ^ d: shop-money.el:1

(defvar shop-items nil "The cart.")

(defun shop-label (cart)
  "Describe CART.
(defun shop-fake () 1)"
  (let ((total (shop-money-total cart)))
;                                  ^ d: shop.el:7
; status: local
    (format "Total: %s" (shop-money-format total))))
;                                ^ d: shop-money.el:2
; status: shop-money-format: by name, 1 match
;                                             ^ d: shop.el:10
; status: local

(defmacro shop-with-cart (&rest body) `(progn ,@body))
(defsubst shop-empty-p (cart) (null cart))
(defcustom shop-currency "$" "Currency." :type 'string)
(defconst shop-tax 0.2)
(defvar-local shop-local-cart nil)
(defface shop-face '((t)) "Face.")
(defgroup shop nil "The shop.")
(defalias 'shop-total #'shop-money-total)
(define-error 'shop-error "Shop error")
(define-minor-mode shop-mode "Shop mode.")
(define-derived-mode shop-list-mode tabulated-list-mode "Shop")
(cl-defstruct (shop-order (:constructor shop-make-order)) id)
(cl-defgeneric shop-price (item))
(cl-defmethod shop-price ((item shop-order)) 1)
(transient-define-prefix shop-menu () "Menu.")
(defadvice shop-label (around shop-trace) ad-do-it)
#| not a block comment in Emacs Lisp |#

(defun shop-run ()
  (shop-with-cart (shop-empty-p shop-items) shop-currency shop-tax shop-local-cart)
;         ^ d: shop.el:19
;                        ^ d: shop.el:20
;                                                 ^ d: shop.el:21
;                                                             ^ d: shop.el:22
;                                                                         ^ d: shop.el:23
;                                    ^ d: shop.el:5
  (list 'shop-face (shop-total nil) (signal 'shop-error nil) (shop-mode) (shop-list-mode))
;            ^ d: shop.el:24
;                        ^ d: shop.el:26
;                                                 ^ d: shop.el:27
;                                                                  ^ d: shop.el:28
;                                                                                ^ d: shop.el:29
  (shop-price (shop-order-id nil)) (shop-menu) #'shop-label :shop-label (shop-fake))
;       ^ d: picker shop.el:31, shop.el:32
;                    ^ d: none
;                                       ^ d: shop.el:33
;                                                    ^ d: shop.el:7
;                                                                ^ d: shop.el:7
;                                                                            ^ d: none
(shop-label ?\" "(defun shop-quoted () 1)")
;     ^ d: shop.el:7
