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
(defun shop-if (x) (if-let ((shop-tally x)) shop-tally 0))
;                                           ^ d: shop-rebound.el:14
(defun shop-loop (x) (cl-loop for shop-tally in x collect shop-tally))
;                                                         ^ d: shop-rebound.el:16
(defun shop-guard (x) (condition-case shop-tally (shop-tally) (error shop-tally)))
;                                                 ^ d: shop-rebound.el:1
;                                                                    ^ d: shop-rebound.el:18
(defun shop-if2 (x) (if-let ((shop-tally x)) (list shop-tally) (shop-tally)))
;                                                  ^ d: shop-rebound.el:21
;                                                               ^ d: shop-rebound.el:1
(defun shop-pl (x) (pcase-let ((`(,shop-tally ,rate) x)) (list shop-tally)))
;                                                              ^ d: shop-rebound.el:24
(defun shop-pq (x) (pcase-let ((`(shop-tally ,rate) x)) (shop-tally)))
;                                                        ^ d: shop-rebound.el:1
(defun shop-pd (x) (pcase-let ((`(,a . ,shop-tally) x)) (list shop-tally)))
;                                                             ^ d: shop-rebound.el:28
(defun shop-pp (x) (pcase-let ((`(,a ,(pred shop-tally)) x)) (shop-tally)))
;                                                             ^ d: shop-rebound.el:1
(defun shop-ps (x) (pcase-let* ((`(,shop-tally ,b) (shop-tally))) (list shop-tally)))
;                                                   ^ d: shop-rebound.el:1
;                                                                       ^ d: shop-rebound.el:32
(defun shop-pc (x)
  (pcase-let ((`(,shop-tally ,b) x))
    (list shop-tally))
;         ^ d: shop-rebound.el:36
  (shop-tally))
;  ^ d: shop-rebound.el:1
