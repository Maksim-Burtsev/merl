(defun shop-tally () 0)
(defun shop-sum (xs)
  (dolist (shop-tally xs)
    (message "%s" shop-tally))
;                 ^ d: shop-rebound.el:3
  (shop-tally))
;  ^ d: shop-rebound.el:1
(defun shop-twice () (cl-flet ((shop-tally () 2)) (shop-tally)))
;                                                  ^ d: shop-rebound.el:8
(defun shop-count () (dotimes (shop-tally 3) shop-tally) (shop-tally))
;                                                         ^ d: shop-rebound.el:1
(defun shop-split (x) (seq-let (shop-tally rest) x shop-tally))
;                                                  ^ d: shop-rebound.el:12
