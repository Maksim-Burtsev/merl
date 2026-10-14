;;;; A small ledger: entries, totals and a report.
#| A block comment
   over two lines. |#
(defpackage #:ledger
  (:use #:cl)
  (:export #:make-entry #:total #:report))

(in-package #:ledger)

(defconstant +max-entries+ 1024)
(defparameter *ratio* 1.5e3)
(defvar *half* 1/2)
(defvar *mask* #xff)
(defvar *letter* #\z)
(defvar *flags* '(t nil))
(defvar *name* "ledger \"main\"")
(defvar *tags* #(food rent "misc"))

(defstruct entry
  (name "" :type string)
  (amount 0 :type integer))

(defclass account ()
  ((owner :initarg :owner :accessor account-owner)
   (entries :initform nil :accessor account-entries)))

(defgeneric balance (thing))

(defmethod balance ((a account))
  (total (account-entries a)))

(defun credit-p (e)
  (and (entry-p e) (> (entry-amount e) 0)))

(defun total (entries &key (start 0) &aux (sum start))
  (dolist (e entries sum)
    (incf sum (entry-amount e))))

(defun describe-entry (e)
  (cond ((credit-p e) "credit")
        ((zerop (entry-amount e)) "empty")
        (t "debit")))

(defun report (entries &optional (stream *standard-output*))
  (let* ((n (length entries))
         (sum (total entries)))
    (when (> n +max-entries+)
      (error "too many entries: ~a" n))
    (loop for e in entries
          for i from 1
          do (format stream "~d ~a ~,2f~%" i (entry-name e) (entry-amount e))
          collect (entry-amount e) into amounts
          finally (return `(total ,sum amounts ,@amounts)))))

(defmacro swap! (a b)
  (let ((tmp (gensym)))
    `(let ((,tmp ,a)) (setf ,a ,b) (setf ,b ,tmp))))

#+sbcl (defun only-sbcl () 'sbcl)
(defvar *square* (lambda (x) (* x x)))
(mapcar #'describe-entry (list (make-entry :name "a" :amount 1)))
(multiple-value-bind (q r) (floor 7 2)
  (list q r cl:most-positive-fixnum))
