#+nil
(defun husk-off () 1)
#+(or)
(progn
  (defun husk-or () 1))
#-(and)
(defun husk-and () 1)
#+nil
(defun husk-real () 0)
(defun husk-real () 1)
#+sbcl
(defun husk-sbcl () 1)
(defun husk-use ()
  (husk-off)
;  ^ d: none
  (husk-or)
;  ^ d: none
  (husk-and)
;  ^ d: none
  (husk-real)
;  ^ d: husk.lisp:10
  (husk-sbcl))
;  ^ d: husk.lisp:12
