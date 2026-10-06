;;; shop-limit.el --- a when-let* sees its earlier bindings (#731)  -*- lexical-binding: t -*-
(defun shop-limit-double ()
  (let ((limit 10))
    (when-let* ((limit limit)
                (limit (* 2 limit)))
;                           ^ d: shop-limit.el:3; want shop-limit.el:4 (#731)
      limit)))
