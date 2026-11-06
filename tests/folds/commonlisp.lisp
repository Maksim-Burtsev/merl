(defpackage :example  ; f: 1-3
  (:use :cl)  ; f: 1-3
  (:export #:greet))  ; f: 1-3

(in-package :example)  ; f: none

(defun greet (name)  ; f: 7-11
  "Says hello ( to [ someone."
  (let ((s (format nil "hi ~a" name)))  ; f: 7-11
    (print (list #\( #\) #\" #\;))  ; f: 7-11
    s))  ; f: 7-11
  ; f: none
#|
(defun commented-out ()
  1)
|#

(defmacro unless* (test &body body)  ; f: 18-20
  `(if ,test nil  ; f: 18-20
       (progn ,@body)))  ; f: 18-20

#+sbcl  ; f: none
(defun only-on-sbcl ()  ; f: none
  1)  ; f: none

#-(or sbcl  ; f: none
      ccl)  ; f: none
(defvar *elsewhere*  ; f: none
  2)  ; f: none

'(quoted  ; f: none
  list)  ; f: none

#'(lambda ()  ; f: none
    3)  ; f: none

(loop for x in '(1 2 3)  ; f: 37-38
      collect x)  ; f: 37-38

(defclass point ()  ; f: 40-42
  ((x :initarg :x)  ; f: 40-42
   (y :initarg :y)))  ; f: 40-42
