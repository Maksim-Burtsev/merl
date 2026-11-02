(defclass gadget () ())
(defgeneric tune-gadget (gadget level))
;                        ^ d: generic.lisp:2
(defgeneric shine (gadget level)
  (:method ((g gadget) (level integer))
    (list g level))
;           ^ d: generic.lisp:5
  (:method (g level)
    (list g gadget)))
;           ^ d: generic.lisp:1
