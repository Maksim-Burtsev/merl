;;;; loop.lisp: a loop variable is a local (#734)
(in-package :shop)
(defclass line () ())
(defun shop-lines (data)
  (loop for line in data
        collect (first line)))
;                      ^ d: loop.lisp:5
(defun shop-pairs (pairs)
  (loop for (key value) in pairs
        with total = 0
        collect (list value total)))
;                     ^ d: loop.lisp:9
;                           ^ d: loop.lisp:10
