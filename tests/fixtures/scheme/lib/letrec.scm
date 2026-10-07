(define (twin) 0)
(define (pair) (letrec ((one (lambda () (twin))) (twin (lambda () 1))) (one)))
;                                        ^ d: lib/letrec.scm:2
