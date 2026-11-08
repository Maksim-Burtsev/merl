#lang at-exp racket
;; siphon the tea before it cools
;  ^ d: none
(define (pour-off pot)
  (send pot siphon 3)
;           ^ d: none
  (error 'pour-off "cannot siphon twice")
;                           ^ d: none
  (siphon pot))
;   ^ d: siphon.rkt:2
#|
  the tea will siphon slowly
;              ^ d: none
|#
(define-syntax (siphoner stx)
  (with-syntax ([siphon #'pot])
    #'(siphon)))
;       ^ d: siphoning.rkt:16
(define (siphon-doc)
  @list{Tea will siphon through @racket[siphon] in time.})
;                 ^ d: none
;                                         ^ d: siphon.rkt:2
#;(siphon 1)
;   ^ d: siphon.rkt:2
