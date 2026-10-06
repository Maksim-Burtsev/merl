(define (positive x)
  (and-let* (x ((> x 0)))
;                  ^ d: lib/and-let.scm:1
    x))
;   ^ d: lib/and-let.scm:1
