;;; shop-limit.el --- a when-let* sees its earlier bindings (#731)  -*- lexical-binding: t -*-
(defun shop-limit-double ()
  (let ((limit 10))
    (when-let* ((limit limit)
;                      ^ d: shop-limit.el:3
                (limit (* 2 limit)))
;                           ^ d: shop-limit.el:4
      limit)))
;     ^ d: shop-limit.el:6
(defun shop-limit-pick ()
  (let* ((limit 1)
         (pick (lambda (limit) (* limit 2))))
;                                 ^ d: shop-limit.el:12
    (funcall pick limit)))
;                 ^ d: shop-limit.el:11
