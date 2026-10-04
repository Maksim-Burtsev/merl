(in-package :shop-money)
;                 ^ d: package.lisp:2

(defvar *rate* 100)
(defparameter *currency* "$")
(defconstant +tax+ 2)
(defun format-price (cents)
  "Render CENTS.
(defun fake-price () 0)"
  (format nil "~a~,2f" *currency* (/ cents *rate*)))
;                           ^ d: money.lisp:5
;                                             ^ d: money.lisp:4
;                                       ^ d: money.lisp:7
; status: local
(defmacro with-cents ((var) &body body) `(let ((,var 1)) ,@body))
(defgeneric price (item))
(defmethod price ((item integer)) item)
(defclass order () ((id :initarg :id)))
(defstruct (point (:constructor make-pt)) x y)
(defstruct parcel weight)
(deftype cents () 'integer)
(define-condition shop-error (error) ())
(define-compiler-macro format-price (&whole form cents) form)
#|
(defun hidden () 1)
#| nested |#
(defun still-hidden () 2)
|#
