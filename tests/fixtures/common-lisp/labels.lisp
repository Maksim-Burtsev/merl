(defun twin () 0)
(defun pair () (labels ((one () (twin)) (twin () 1)) (one)))
;                                ^ d: labels.lisp:2
