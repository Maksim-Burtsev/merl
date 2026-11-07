(define (scm-extend a b) a)
(define (go p d)
  (match p d (scm-extend 1 d)))
;             ^ d: matcher.scm:1
(define (norm) (match-lambda (scm-extend 1)))
;                             ^ d: matcher.scm:1
(define (one x) (match-define scm-extend x) (scm-extend))
;                                            ^ d: matcher.scm:1
