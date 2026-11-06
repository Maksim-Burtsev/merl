(defun tally () 0)
(defun sum-all (xs)
  (dolist (tally xs)
    (print tally))
;          ^ d: rebound.lisp:3
  (tally))
;  ^ d: rebound.lisp:1
(defun split-pair (x) (destructuring-bind (tally rest) x (list tally rest)))
;                                                              ^ d: rebound.lisp:8
(defun pick (x) (multiple-value-bind (a b) x (tally)))
;                                             ^ d: rebound.lisp:1
(defun count-up (x) (do ((tally 0 (1+ tally))) ((> tally x)) (print tally)))
;                                                                   ^ d: rebound.lisp:12
(defun slots (x) (with-slots (tally) x tally))
;                                      ^ d: rebound.lisp:14
(defun quiet () (symbol-macrolet ((tally 1)) tally) "tally" (tally))
;                                                            ^ d: rebound.lisp:1
(defun handle (x) (handler-case (tally) (error (tally) tally)))
;                                ^ d: rebound.lisp:1
;                                                      ^ d: rebound.lisp:18
