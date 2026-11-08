(defun
  kettle-boil (x) (* 2 x))
(DEFUN
  ; kettle-boil
  kettle-cool (x) x)
(defun kettle-use ()
  (kettle-boil 1)
;  ^ d: kettle.lisp:1
  (kettle-cool 2))
;  ^ d: kettle.lisp:3
