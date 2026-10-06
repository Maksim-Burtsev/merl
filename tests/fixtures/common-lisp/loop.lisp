;;;; loop.lisp: a loop variable is a local (#734)
(in-package :shop)
(defclass line () ())
(defun shop-lines (data)
  (loop for line in data
        collect (first line)))
;                      ^ d: loop.lisp:3; want loop.lisp:5 (#734)
