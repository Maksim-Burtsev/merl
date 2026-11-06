(define (tally) 0)
(define (split x)
  (receive (tally rest) (values x x)
    (display tally))
;            ^ d: lib/rebound.scm:3
  (tally))
;  ^ d: lib/rebound.scm:1
(define (careful x) (guard (tally (#t tally)) x))
;                                     ^ d: lib/rebound.scm:8
(define (careless x) (guard (e (#t (tally))) x))
;                                   ^ d: lib/rebound.scm:1
(define (count-up x) (do ((tally 0 (+ tally 1))) ((> tally x)) (display tally)))
;                                                                       ^ d: lib/rebound.scm:12
(define (inner x) (define (tally) 2) (tally))
;                                     ^ d: lib/rebound.scm:14
(define (arities x) (case-lambda ((tally) tally) ((a) (tally))))
;                                         ^ d: lib/rebound.scm:16
;                                                      ^ d: lib/rebound.scm:1
(define (rest-of x) (lambda tally tally))
;                                 ^ d: lib/rebound.scm:19
(define (plain x) (lambda (y) y) (tally))
;                                 ^ d: lib/rebound.scm:1
